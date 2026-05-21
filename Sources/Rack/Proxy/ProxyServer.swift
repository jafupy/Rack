import Foundation
import Dispatch
@preconcurrency import NIOCore
import NIOPosix
@preconcurrency import NIOHTTP1
@preconcurrency import NIOWebSocket
@preconcurrency import NIOSSL
import RackCoreFFI

/// HTTP/1.1 reverse proxy that routes *.localhost to unix sockets.
/// Runs inside Rack.app — no external daemon, no Node.
final class ProxyServer: @unchecked Sendable {
    static let defaultPort = 1355
    static let defaultTLSPort = 1443
    private static let daemonPath = "/Library/LaunchDaemons/com.jafupy.Rack.portfwd.plist"

    // Set after start() binds. Read by Models.localURL and NameInferrer.
    static nonisolated(unsafe) var boundPort: Int = defaultPort
    static nonisolated(unsafe) var boundTLSPort: Int = defaultTLSPort

    private let group = MultiThreadedEventLoopGroup(numberOfThreads: 2)
    private var channels: [any Channel] = []

    func start(port: Int = ProxyServer.defaultPort) async throws {
        let httpBootstrap = makeBootstrap()

        var lastError: Error?
        for candidate in port...(port + 10) {
            do {
                let bound = try await bindPair(bootstrap: httpBootstrap, port: candidate)
                channels.append(contentsOf: bound)
                ProxyServer.boundPort = candidate
                break
            } catch {
                lastError = error
            }
        }

        guard !channels.isEmpty else {
            throw lastError ?? ProxyError.backendUnavailable
        }

        do {
            let tlsContext = try Self.makeTLSContext()
            let httpsBootstrap = makeBootstrap(tlsContext: tlsContext)
            var tlsLastError: Error?
            for candidate in Self.defaultTLSPort...(Self.defaultTLSPort + 10) {
                do {
                    let bound = try await bindPair(bootstrap: httpsBootstrap, port: candidate)
                    channels.append(contentsOf: bound)
                    ProxyServer.boundTLSPort = candidate
                    break
                } catch {
                    tlsLastError = error
                }
            }
            if ProxyServer.boundTLSPort == Self.defaultTLSPort && !channels.contains(where: { channel in
                guard let address = channel.localAddress else { return false }
                return address.port == Self.defaultTLSPort
            }) {
                throw tlsLastError ?? TLSCertificateError.creationFailed
            }
        } catch {
            print("RackProxy HTTPS listener disabled: \(error)")
        }

        // Sync the UserDefaults "standard ports" flag with the actual daemon file on disk.
        // The file persists across reboots; UserDefaults might have drifted.
        let daemonExists = FileManager.default.fileExists(atPath: ProxyServer.daemonPath)
        UserDefaults.standard.set(daemonExists, forKey: "standardPortsEnabled")
    }

    private func makeBootstrap(tlsContext: NIOSSLContext? = nil) -> ServerBootstrap {
        ServerBootstrap(group: group)
            .serverChannelOption(.backlog, value: 256)
            .serverChannelOption(.socketOption(.so_reuseaddr), value: 1)
            .childChannelInitializer { channel in
                let upgrader = NIOWebSocketServerUpgrader(
                    shouldUpgrade: { channel, head in
                        guard rackRoute(for: head.headers["host"].first) != nil else {
                            return channel.eventLoop.makeSucceededFuture(nil)
                        }
                        var responseHeaders = HTTPHeaders()
                        if let webSocketProtocol = head.headers["sec-websocket-protocol"].first {
                            responseHeaders.add(name: "sec-websocket-protocol", value: webSocketProtocol)
                        }
                        return channel.eventLoop.makeSucceededFuture(responseHeaders)
                    },
                    upgradePipelineHandler: { channel, head in
                        WebSocketBackendConnector.connect(frontend: channel, head: head)
                    }
                )
                let upgradeConfig: NIOHTTPServerUpgradeConfiguration = (
                    upgraders: [upgrader],
                    completionHandler: { _ in }
                )
                let configuredTLS: EventLoopFuture<Void>
                if let tlsContext {
                    configuredTLS = channel.pipeline.addHandler(NIOSSLServerHandler(context: tlsContext))
                } else {
                    configuredTLS = channel.eventLoop.makeSucceededVoidFuture()
                }

                return configuredTLS.flatMap {
                    channel.pipeline.configureHTTPServerPipeline(
                    withPipeliningAssistance: true,
                    withServerUpgrade: upgradeConfig,
                    withErrorHandling: true
                    )
                }.flatMap {
                    channel.pipeline.addHandler(HTTPProxyHandler())
                }
            }
            .childChannelOption(.socketOption(.so_reuseaddr), value: 1)
            .childChannelOption(.maxMessagesPerRead, value: 16)
    }

    private func bindPair(bootstrap: ServerBootstrap, port: Int) async throws -> [any Channel] {
        let ipv4 = try await bootstrap.bind(host: "127.0.0.1", port: port).get()
        do {
            let ipv6 = try await bootstrap.bind(host: "::1", port: port).get()
            return [ipv4, ipv6]
        } catch {
            try? await ipv4.close().get()
            throw error
        }
    }

    func stop() async throws {
        for channel in channels {
            try await channel.close()
        }
        channels = []
        try await group.shutdownGracefully()
    }

    // MARK: - Standard port forwarding (pfctl, requires administrator)

    /// Installs a LaunchDaemon that redirects standard localhost web ports to Rack's proxy
    /// and applies the rules immediately. Uses the com.apple/rack anchor so it doesn't wipe
    /// macOS system pf rules.
    /// Shows macOS authentication dialog. Returns true on success.
    @discardableResult
    static func setupPortForwarding() -> Bool {
        let certPath: String
        do {
            certPath = try ensureLocalTLSCertificate().certificate
        } catch {
            return false
        }

        let rules = """
            rdr pass on lo0 proto tcp from any to any port 80 -> 127.0.0.1 port \(defaultPort)
            rdr pass on lo0 proto tcp from any to any port 443 -> 127.0.0.1 port \(defaultTLSPort)
            """
        let pfCommand = rules
            .split(separator: "\n")
            .map { "'\($0)'" }
            .joined(separator: " ")

        let plist = """
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
            <dict>
                <key>Label</key>
                <string>com.jafupy.Rack.portfwd</string>
                <key>ProgramArguments</key>
                <array>
                    <string>/bin/sh</string>
                    <string>-c</string>
                    <string>printf '%s\\n' \(pfCommand) | /sbin/pfctl -a com.apple/rack -f - 2&gt;/dev/null || true</string>
                </array>
                <key>RunAtLoad</key>
                <true/>
            </dict>
            </plist>
            """

        let tmpPath = "/tmp/com.jafupy.Rack.portfwd.plist"
        guard (try? plist.write(toFile: tmpPath, atomically: true, encoding: .utf8)) != nil else {
            return false
        }

        let escapedCertPath = shellEscape(certPath)
        // Install daemon AND apply the rule immediately — no reboot required.
        let script = """
            do shell script "cp '\(tmpPath)' '\(daemonPath)' && launchctl bootstrap system '\(daemonPath)' 2>/dev/null || true; /usr/bin/security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain \(escapedCertPath) 2>/dev/null || true; printf '%s\\n' \(pfCommand) | /sbin/pfctl -a com.apple/rack -f -" with administrator privileges
            """
        var error: NSDictionary?
        NSAppleScript(source: script)?.executeAndReturnError(&error)

        let ok = error == nil
        UserDefaults.standard.set(ok, forKey: "standardPortsEnabled")
        return ok
    }

    /// Removes the port forwarding LaunchDaemon and immediately flushes the pf anchor.
    static func teardownPortForwarding() {
        let script = """
            do shell script "launchctl bootout system '\(daemonPath)' 2>/dev/null; /sbin/pfctl -a com.apple/rack -F all 2>/dev/null; rm -f '\(daemonPath)'; true" with administrator privileges
            """
        var error: NSDictionary?
        NSAppleScript(source: script)?.executeAndReturnError(&error)
        UserDefaults.standard.set(false, forKey: "standardPortsEnabled")
    }

    private static func shellEscape(_ value: String) -> String {
        "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }

    private enum TLSCertificateError: Error {
        case creationFailed
    }

    private static func makeTLSContext() throws -> NIOSSLContext {
        let paths = try ensureLocalTLSCertificate()
        let certs = try NIOSSLCertificate.fromPEMFile(paths.certificate)
        let key = try NIOSSLPrivateKey(file: paths.privateKey, format: .pem)
        let configuration = TLSConfiguration.makeServerConfiguration(
            certificateChain: certs.map { .certificate($0) },
            privateKey: .privateKey(key)
        )
        return try NIOSSLContext(configuration: configuration)
    }

    private static func ensureLocalTLSCertificate() throws -> (certificate: String, privateKey: String) {
        let tlsDir = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".config/rack/tls")
        try FileManager.default.createDirectory(at: tlsDir, withIntermediateDirectories: true)

        let certPath = tlsDir.appending(path: "localhost.pem").path
        let keyPath = tlsDir.appending(path: "localhost-key.pem").path
        if FileManager.default.fileExists(atPath: certPath),
           FileManager.default.fileExists(atPath: keyPath) {
            return (certPath, keyPath)
        }

        let configPath = tlsDir.appending(path: "localhost-openssl.cnf").path
        let config = """
            [req]
            distinguished_name=req_distinguished_name
            x509_extensions=v3_req
            prompt=no
            [req_distinguished_name]
            CN=*.localhost
            [v3_req]
            keyUsage=critical,digitalSignature,keyEncipherment
            extendedKeyUsage=serverAuth
            subjectAltName=@alt_names
            [alt_names]
            DNS.1=localhost
            DNS.2=*.localhost
            """
        try config.write(toFile: configPath, atomically: true, encoding: .utf8)

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
        process.arguments = [
            "req", "-x509", "-newkey", "rsa:2048", "-sha256", "-days", "825", "-nodes",
            "-keyout", keyPath,
            "-out", certPath,
            "-config", configPath,
        ]
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw TLSCertificateError.creationFailed
        }
        return (certPath, keyPath)
    }

    deinit {
        try? group.syncShutdownGracefully()
    }
}

private func rackRoute(for host: String?) -> Route? {
    guard let name = rackRouteName(from: host) else { return nil }
    return RouteRegistry.shared.route(for: name)
}

private func isRackLocalHost(_ host: String?) -> Bool {
    rackHostname(from: host) == "rack.local"
}

private func rackRouteName(from host: String?) -> String? {
    guard let hostname = rackHostname(from: host), hostname.hasSuffix(".localhost") else { return nil }
    let name = String(hostname.dropLast(".localhost".count))
    return name.isEmpty ? nil : name
}

private func rackHostname(from host: String?) -> String? {
    guard let host, !host.isEmpty else { return nil }
    if host.first == "[" {
        return host.dropFirst().split(separator: "]", maxSplits: 1).first.map { String($0).lowercased() }
    }
    return host.split(separator: ":", maxSplits: 1).first.map { String($0).lowercased() }
}

private enum RackLocalFunctionError: Error {
    case invalidResponse
}

private struct RackLocalFunctionResponse: Sendable {
    var statusCode: Int
    var headers: [String: String]
    var body: String
}

private struct RackLocalResponseContext: @unchecked Sendable {
    var context: ChannelHandlerContext
}

private final class RackLocalFunctionThreadLimiter: @unchecked Sendable {
    static let shared = RackLocalFunctionThreadLimiter()

    private let condition = NSCondition()
    private var activeCount = 0

    private var maxThreads: Int {
        let configured = UserDefaults.standard.integer(forKey: "functionWorkerLimit")
        return min(max(configured == 0 ? 4 : configured, 1), 32)
    }

    private init() {}

    func run<T>(_ work: () -> T) -> T {
        condition.lock()
        while activeCount >= maxThreads {
            condition.wait()
        }
        activeCount += 1
        condition.unlock()

        defer {
            condition.lock()
            activeCount -= 1
            condition.broadcast()
            condition.unlock()
        }

        return work()
    }
}

// MARK: - WebSocket tunnel

private enum WebSocketBackendConnector {
    static func connect(frontend: any Channel, head: HTTPRequestHead) -> EventLoopFuture<Void> {
        guard let route = rackRoute(for: head.headers["host"].first),
              !route.socketPath.isEmpty || route.tcpPort > 0
        else {
            return frontend.eventLoop.makeFailedFuture(ProxyError.backendUnavailable)
        }

        let tunnel = WebSocketTunnel(eventLoop: frontend.eventLoop)
        let requestKey = head.headers["sec-websocket-key"].first
            ?? NIOWebSocketClientUpgrader.randomRequestKey()
        let upgrader = NIOWebSocketClientUpgrader(requestKey: requestKey) { backend, _ in
            backend.pipeline.addHandler(WebSocketFrameRelay(tunnel: tunnel, side: .backend)).map {
                tunnel.setBackend(backend)
            }
        }
        let bootstrap = ClientBootstrap(group: frontend.eventLoop)
            .channelInitializer { channel in
                let requestHandler = WebSocketUpgradeRequestHandler(head: head)
                let upgradeConfig: NIOHTTPClientUpgradeConfiguration = (
                    upgraders: [upgrader],
                    completionHandler: { context in
                        context.pipeline.removeHandler(requestHandler, promise: nil)
                    }
                )
                return channel.pipeline.addHTTPClientHandlers(withClientUpgrade: upgradeConfig).flatMap {
                    channel.pipeline.addHandler(requestHandler)
                }
            }

        let connectFuture = route.socketPath.isEmpty
            ? bootstrap.connect(host: "127.0.0.1", port: route.tcpPort)
            : bootstrap.connect(unixDomainSocketPath: route.socketPath)

        return connectFuture.flatMap { backend in
            tunnel.setFrontend(frontend)
            return frontend.pipeline.addHandler(WebSocketFrameRelay(tunnel: tunnel, side: .frontend)).map {
                _ = backend
            }
        }
    }
}

private enum ProxyError: Error {
    case backendUnavailable
}

private final class WebSocketUpgradeRequestHandler: ChannelInboundHandler, RemovableChannelHandler, @unchecked Sendable {
    typealias InboundIn = HTTPClientResponsePart
    typealias OutboundOut = HTTPClientRequestPart

    private var head: HTTPRequestHead

    init(head: HTTPRequestHead) {
        self.head = head
    }

    func channelActive(context: ChannelHandlerContext) {
        head.headers.remove(name: "connection")
        head.headers.remove(name: "upgrade")
        head.headers.remove(name: "sec-websocket-key")
        head.headers.remove(name: "sec-websocket-version")
        context.write(wrapOutboundOut(.head(head)), promise: nil)
        context.writeAndFlush(wrapOutboundOut(.end(nil)), promise: nil)
    }
}

private final class WebSocketTunnel: @unchecked Sendable {
    enum Side {
        case frontend
        case backend
    }

    private let eventLoop: any EventLoop
    private var frontend: (any Channel)?
    private var backend: (any Channel)?
    private var pendingFrontendFrames: [WebSocketFrame] = []
    private var pendingBackendFrames: [WebSocketFrame] = []

    init(eventLoop: any EventLoop) {
        self.eventLoop = eventLoop
    }

    func setFrontend(_ channel: any Channel) {
        eventLoop.execute {
            self.frontend = channel
            self.flushPending()
        }
    }

    func setBackend(_ channel: any Channel) {
        eventLoop.execute {
            self.backend = channel
            self.flushPending()
        }
    }

    func relay(_ frame: WebSocketFrame, from side: Side) {
        eventLoop.execute {
            switch side {
            case .frontend:
                if let backend = self.backend {
                    backend.writeAndFlush(frame, promise: nil)
                } else {
                    self.pendingFrontendFrames.append(frame)
                }
            case .backend:
                if let frontend = self.frontend {
                    frontend.writeAndFlush(frame, promise: nil)
                } else {
                    self.pendingBackendFrames.append(frame)
                }
            }
        }
    }

    func close(from side: Side) {
        eventLoop.execute {
            switch side {
            case .frontend:
                self.backend?.close(promise: nil)
            case .backend:
                self.frontend?.close(promise: nil)
            }
        }
    }

    private func flushPending() {
        guard let frontend, let backend else { return }
        for frame in pendingFrontendFrames {
            backend.write(frame, promise: nil)
        }
        pendingFrontendFrames.removeAll()
        for frame in pendingBackendFrames {
            frontend.write(frame, promise: nil)
        }
        pendingBackendFrames.removeAll()
        backend.flush()
        frontend.flush()
    }
}

private final class WebSocketFrameRelay: ChannelInboundHandler, @unchecked Sendable {
    typealias InboundIn = WebSocketFrame
    typealias OutboundOut = WebSocketFrame

    private let tunnel: WebSocketTunnel
    private let side: WebSocketTunnel.Side

    init(tunnel: WebSocketTunnel, side: WebSocketTunnel.Side) {
        self.tunnel = tunnel
        self.side = side
    }

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        tunnel.relay(unwrapInboundIn(data), from: side)
    }

    func channelInactive(context: ChannelHandlerContext) {
        tunnel.close(from: side)
    }

    func errorCaught(context: ChannelHandlerContext, error: Error) {
        tunnel.close(from: side)
        context.close(promise: nil)
    }
}

// MARK: - Proxy handler

private final class HTTPProxyHandler: ChannelInboundHandler, @unchecked Sendable {
    typealias InboundIn = HTTPServerRequestPart
    typealias OutboundOut = HTTPServerResponsePart

    private var pendingHead: HTTPRequestHead?
    private var bodyBuffer: ByteBuffer?
    private var pendingEnd: HTTPHeaders?
    private var backendChannel: (any Channel)?
    private var rackLocalHead: HTTPRequestHead?

    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        switch unwrapInboundIn(data) {
        case .head(let head):
            pendingHead = head
            if isRackLocalHost(head.headers["host"].first) {
                rackLocalHead = head
                return
            }

            guard resolve(host: head.headers["host"].first) != nil else {
                sendError(context: context, status: .badGateway,
                          body: "rack: no route for \(head.headers["host"].first ?? "unknown")")
                return
            }

            if isLoopback(head.headers["host"].first) {
                sendError(context: context, status: .custom(code: 508, reasonPhrase: "Loop Detected"),
                          body: "rack: proxy loop detected — check your Vite/webpack proxy config and set changeOrigin: true")
                return
            }

            openBackend(context: context, host: head.headers["host"].first ?? "", head: head)

        case .body(let buf):
            if let backend = backendChannel {
                writeBackend(backend, .body(.byteBuffer(buf)), flush: true)
            } else {
                appendToBodyBuffer(buf)
            }

        case .end(let trailers):
            if let rackLocalHead {
                pendingEnd = trailers
                handleRackLocal(context: context, head: rackLocalHead)
                return
            }

            if let backend = backendChannel {
                writeBackend(backend, .end(trailers), flush: true)
            } else {
                pendingEnd = trailers
            }
        }
    }

    func errorCaught(context: ChannelHandlerContext, error: Error) {
        backendChannel?.close(promise: nil)
        context.close(promise: nil)
    }

    // MARK: Private

    private func resolve(host: String?) -> Route? {
        rackRoute(for: host)
    }

    private func handleRackLocal(context: ChannelHandlerContext, head: HTTPRequestHead) {
        let path = normalizeRackLocalPath(head.uri)
        if path == "/" {
            sendPlainResponse(
                context: context,
                status: .ok,
                body: "Rack.\n",
                headers: ["content-type": "text/plain"]
            )
            return
        }

        if path.starts(with: "/_") {
            sendError(context: context, status: .notFound, body: "rack: reserved path")
            return
        }

        let body = bodyBuffer.map { String(buffer: $0) } ?? ""
        let request: [String: Any] = [
            "type": "function.http",
            "payload": [
                "method": head.method.rawValue,
                "path": path,
                "uri": head.uri,
                "headers": requestHeaders(from: head.headers),
                "body": body,
            ],
        ]

        guard let data = try? JSONSerialization.data(withJSONObject: request),
              let command = String(data: data, encoding: .utf8)
        else {
            sendError(context: context, status: .internalServerError, body: "rack: function dispatch failed")
            return
        }

        let responseContext = RackLocalResponseContext(context: context)
        DispatchQueue.global(qos: .userInitiated).async {
            let result = RackLocalFunctionThreadLimiter.shared.run {
                self.dispatchRackLocalFunction(command)
            }
            responseContext.context.eventLoop.execute {
                switch result {
                case .success(let response):
                    let status = HTTPResponseStatus(statusCode: response.statusCode)
                    self.sendPlainResponse(
                        context: responseContext.context,
                        status: status,
                        body: response.body,
                        headers: response.headers
                    )

                case .failure:
                    self.sendError(
                        context: responseContext.context,
                        status: .internalServerError,
                        body: "rack: function dispatch failed"
                    )
                }
            }
        }
    }

    private func isLoopback(_ host: String?) -> Bool {
        guard let hostname = rackHostname(from: host), hostname.hasSuffix(".localhost") else { return false }
        guard let name = rackRouteName(from: host) else { return false }
        return RouteRegistry.shared.route(for: name) == nil
    }

    /// Resolves the host on every attempt so the proxy picks up the port as soon as
    /// awaitServerReady discovers it. Waits up to 60 s total (120 × 500 ms).
    private func openBackend(context: ChannelHandlerContext, host: String, head: HTTPRequestHead, attempt: Int = 0) {
        guard let route = resolve(host: host) else {
            sendError(context: context, status: .badGateway,
                      body: "rack: no route for \(host)")
            return
        }

        // Not ready yet — socket path empty and no TCP port
        guard !route.socketPath.isEmpty || route.tcpPort > 0 else {
            if attempt < 120 && context.channel.isActive {
                context.eventLoop.scheduleTask(in: .milliseconds(500)) {
                    self.openBackend(context: context, host: host, head: head, attempt: attempt + 1)
                }
            } else {
                sendError(context: context, status: .serviceUnavailable,
                          body: "rack: server did not start within 60s")
            }
            return
        }

        let bootstrap = ClientBootstrap(group: context.eventLoop)
            .channelInitializer { channel in
                channel.pipeline.addHTTPClientHandlers().flatMap {
                    channel.pipeline.addHandler(
                        BackendResponseHandler(frontend: context.channel)
                    )
                }
            }

        let connectFuture = route.socketPath.isEmpty
            ? bootstrap.connect(host: "127.0.0.1", port: route.tcpPort)
            : bootstrap.connect(unixDomainSocketPath: route.socketPath)

        connectFuture.whenComplete { result in
                switch result {
                case .success(let backend):
                    self.backendChannel = backend
                    var forwardHead = head
                    if forwardHead.headers["host"].isEmpty {
                        forwardHead.headers.add(name: "host", value: host)
                    }
                    self.writeBackend(backend, .head(forwardHead))
                    if let buf = self.bodyBuffer {
                        self.writeBackend(backend, .body(.byteBuffer(buf)))
                    }
                    if let trailers = self.pendingEnd {
                        self.writeBackend(backend, .end(trailers))
                    }
                    backend.flush()

                case .failure:
                    if attempt < 120 && context.channel.isActive {
                        context.eventLoop.scheduleTask(in: .milliseconds(500)) {
                            self.openBackend(context: context, host: host, head: head, attempt: attempt + 1)
                        }
                    } else {
                        self.sendError(context: context, status: .badGateway,
                                       body: "rack: backend not ready — is the server starting?")
                    }
                }
            }
    }

    private func writeBackend(_ backend: any Channel, _ part: HTTPClientRequestPart, flush: Bool = false) {
        if flush {
            backend.writeAndFlush(part, promise: nil)
        } else {
            backend.write(part, promise: nil)
        }
    }

    private func appendToBodyBuffer(_ buffer: ByteBuffer) {
        if bodyBuffer == nil {
            bodyBuffer = buffer
        } else {
            bodyBuffer?.writeImmutableBuffer(buffer)
        }
    }

    private func normalizeRackLocalPath(_ uri: String) -> String {
        let rawPath = uri.split(separator: "?", maxSplits: 1).first.map(String.init) ?? "/"
        var normalized = rawPath.hasPrefix("/") ? rawPath : "/\(rawPath)"
        while normalized.count > 1, normalized.last == "/" {
            normalized.removeLast()
        }
        return normalized
    }

    private func rackCoreCommand(_ json: String) -> String? {
        guard let response = rack_core_command(json) else { return nil }
        defer { rack_core_free_string(response) }
        return String(cString: response)
    }

    private func dispatchRackLocalFunction(_ command: String) -> Result<RackLocalFunctionResponse, RackLocalFunctionError> {
        guard let responseJSON = rackCoreCommand(command),
              let responseData = responseJSON.data(using: .utf8),
              let response = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any],
              let payload = response["payload"] as? [String: Any]
        else {
            return .failure(RackLocalFunctionError.invalidResponse)
        }

        return .success(RackLocalFunctionResponse(
            statusCode: payload["status"] as? Int ?? 500,
            headers: payload["headers"] as? [String: String] ?? ["content-type": "text/plain"],
            body: payload["body"] as? String ?? ""
        ))
    }

    private func requestHeaders(from headers: HTTPHeaders) -> [String: String] {
        var result: [String: String] = [:]
        for header in headers {
            let name = header.name.lowercased()
            if let existing = result[name] {
                result[name] = "\(existing), \(header.value)"
            } else {
                result[name] = header.value
            }
        }
        return result
    }

    private func sendError(context: ChannelHandlerContext, status: HTTPResponseStatus, body: String) {
        sendPlainResponse(
            context: context,
            status: status,
            body: body,
            headers: ["content-type": "text/plain"]
        )
    }

    private func sendPlainResponse(
        context: ChannelHandlerContext,
        status: HTTPResponseStatus,
        body: String,
        headers extraHeaders: [String: String]
    ) {
        var buf = context.channel.allocator.buffer(capacity: body.utf8.count)
        buf.writeString(body)
        var headers = HTTPHeaders()
        for (name, value) in extraHeaders {
            headers.replaceOrAdd(name: name, value: value)
        }
        headers.replaceOrAdd(name: "content-length", value: "\(body.utf8.count)")
        headers.replaceOrAdd(name: "connection", value: "close")
        let head = HTTPResponseHead(version: .http1_1, status: status, headers: headers)
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
            frontend.write(HTTPServerResponsePart.head(responseHead), promise: nil)
        case .body(let buf):
            frontend.write(HTTPServerResponsePart.body(.byteBuffer(buf)), promise: nil)
        case .end(let trailers):
            frontend.writeAndFlush(HTTPServerResponsePart.end(trailers), promise: nil)
        }
    }

    func errorCaught(context: ChannelHandlerContext, error: Error) {
        frontend.close(promise: nil)
        context.close(promise: nil)
    }
}
