import Foundation
import NIOCore
import NIOPosix
import NIOHTTP1

/// HTTP/1.1 reverse proxy that routes *.localhost:1355 to unix sockets.
/// Runs inside Rack.app — no external daemon, no Node.
final class ProxyServer: @unchecked Sendable {
    static let defaultPort = 1355

    private let group = MultiThreadedEventLoopGroup(numberOfThreads: 2)
    private var channel: (any Channel)?

    func start(port: Int = ProxyServer.defaultPort) async throws {
        let bootstrap = ServerBootstrap(group: group)
            .serverChannelOption(.backlog, value: 256)
            .serverChannelOption(.socketOption(.so_reuseaddr), value: 1)
            .childChannelInitializer { channel in
                channel.pipeline.configureHTTPServerPipeline(
                    withPipeliningAssistance: true,
                    withErrorHandling: true
                ).flatMap {
                    channel.pipeline.addHandler(HTTPProxyHandler())
                }
            }
            .childChannelOption(.socketOption(.so_reuseaddr), value: 1)
            .childChannelOption(.maxMessagesPerRead, value: 16)

        // Try the default port then fall back to +1 … +10
        var lastError: Error?
        for candidate in port...(port + 10) {
            do {
                channel = try await bootstrap.bind(host: "127.0.0.1", port: candidate).get()
                return
            } catch {
                lastError = error
            }
        }
        throw lastError!
    }

    func stop() async throws {
        try await channel?.close()
        try await group.shutdownGracefully()
    }

    deinit {
        try? group.syncShutdownGracefully()
    }
}

// MARK: - Proxy handler

private final class HTTPProxyHandler: ChannelInboundHandler, @unchecked Sendable {
    typealias InboundIn = HTTPServerRequestPart
    typealias OutboundOut = HTTPServerResponsePart

    private var pendingHead: HTTPRequestHead?
    private var bodyBuffer: ByteBuffer?
    private var backendChannel: (any Channel)?
    private var isWebSocket = false

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        switch unwrapInboundIn(data) {
        case .head(let head):
            pendingHead = head
            guard let route = resolve(host: head.headers["host"].first) else {
                sendError(context: context, status: .badGateway,
                          body: "rack: no route for \(head.headers["host"].first ?? "unknown")")
                return
            }

            // Loop detection
            if isLoopback(head.headers["host"].first) {
                sendError(context: context, status: .custom(code: 508, reasonPhrase: "Loop Detected"),
                          body: "rack: proxy loop detected — check your Vite/webpack proxy config and set changeOrigin: true")
                return
            }

            isWebSocket = head.headers["upgrade"].first?.lowercased() == "websocket"
            openBackend(context: context, socketPath: route.socketPath, head: head)

        case .body(let buf):
            if let backend = backendChannel {
                backend.writeAndFlush(HTTPClientRequestPart.body(.byteBuffer(buf)), promise: nil)
            } else {
                // Buffer until backend is ready
                if bodyBuffer == nil {
                    bodyBuffer = buf
                } else {
                    bodyBuffer!.writeImmutableBuffer(buf)
                }
            }

        case .end(let trailers):
            backendChannel?.writeAndFlush(HTTPClientRequestPart.end(trailers), promise: nil)
        }
    }

    func errorCaught(context: ChannelHandlerContext, error: Error) {
        backendChannel?.close(promise: nil)
        context.close(promise: nil)
    }

    // MARK: Private

    private func resolve(host: String?) -> Route? {
        guard let host else { return nil }
        // "fix-auth.myapp.localhost:1355" -> "fix-auth.myapp"
        let name = host
            .components(separatedBy: ":").first!
            .replacingOccurrences(of: ".localhost", with: "")
        return RouteRegistry.shared.route(for: name)
    }

    private func isLoopback(_ host: String?) -> Bool {
        guard let host else { return false }
        let name = host
            .components(separatedBy: ":").first!
            .replacingOccurrences(of: ".localhost", with: "")
        // It loops if the resolved route's socket would route back here
        // Simple check: host contains .localhost and we got a route
        return host.hasSuffix(".localhost") && RouteRegistry.shared.route(for: name) == nil
    }

    private func openBackend(context: ChannelHandlerContext, socketPath: String, head: HTTPRequestHead) {
        let clientBootstrap = ClientBootstrap(group: context.eventLoop)
            .channelInitializer { channel in
                if self.isWebSocket {
                    // For WebSocket: raw bytes after upgrade
                    return channel.pipeline.addHandler(
                        BackendRelayHandler(frontend: context.channel)
                    )
                } else {
                    return channel.pipeline.addHTTPClientHandlers().flatMap {
                        channel.pipeline.addHandler(
                            BackendResponseHandler(frontend: context.channel)
                        )
                    }
                }
            }

        clientBootstrap.connect(unixDomainSocketPath: socketPath).whenComplete { result in
            switch result {
            case .success(let backend):
                self.backendChannel = backend

                // Forward the buffered head
                var forwardHead = head
                forwardHead.headers.remove(name: "host")
                backend.write(HTTPClientRequestPart.head(forwardHead), promise: nil)

                // Flush any buffered body
                if let buf = self.bodyBuffer {
                    backend.write(HTTPClientRequestPart.body(.byteBuffer(buf)), promise: nil)
                }
                backend.flush()

            case .failure:
                self.sendError(context: context, status: .badGateway,
                               body: "rack: backend not ready yet — is the server starting?")
            }
        }
    }

    private func sendError(context: ChannelHandlerContext, status: HTTPResponseStatus, body: String) {
        var buf = context.channel.allocator.buffer(capacity: body.utf8.count)
        buf.writeString(body)
        let head = HTTPResponseHead(version: .http1_1, status: status, headers: [
            "content-type": "text/plain",
            "content-length": "\(body.utf8.count)",
            "connection": "close",
        ])
        context.write(wrapOutboundOut(.head(head)), promise: nil)
        context.write(wrapOutboundOut(.body(.byteBuffer(buf))), promise: nil)
        context.writeAndFlush(wrapOutboundOut(.end(nil))).whenComplete { _ in
            context.close(promise: nil)
        }
    }
}

// MARK: - Backend response relay (HTTP mode)

private final class BackendResponseHandler: ChannelInboundHandler, @unchecked Sendable {
    typealias InboundIn = HTTPClientResponsePart
    typealias OutboundOut = HTTPServerResponsePart

    private let frontend: any Channel

    init(frontend: any Channel) {
        self.frontend = frontend
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        switch unwrapInboundIn(data) {
        case .head(let head):
            let responseHead = HTTPResponseHead(version: head.version, status: head.status,
                                               headers: head.headers)
            frontend.write(NIOAny(HTTPServerResponsePart.head(responseHead)), promise: nil)
        case .body(let buf):
            frontend.write(NIOAny(HTTPServerResponsePart.body(.byteBuffer(buf))), promise: nil)
        case .end(let trailers):
            frontend.writeAndFlush(NIOAny(HTTPServerResponsePart.end(trailers)), promise: nil)
        }
    }

    func errorCaught(context: ChannelHandlerContext, error: Error) {
        frontend.close(promise: nil)
        context.close(promise: nil)
    }
}

// MARK: - Raw byte relay (WebSocket / tunnel mode)

private final class BackendRelayHandler: ChannelInboundHandler, @unchecked Sendable {
    typealias InboundIn = ByteBuffer

    private let frontend: any Channel

    init(frontend: any Channel) {
        self.frontend = frontend
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        frontend.writeAndFlush(data, promise: nil)
    }

    func channelInactive(context: ChannelHandlerContext) {
        frontend.close(promise: nil)
    }
}
