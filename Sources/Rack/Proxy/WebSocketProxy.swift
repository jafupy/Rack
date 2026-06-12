@preconcurrency import NIOCore
@preconcurrency import NIOHTTP1
import NIOPosix
@preconcurrency import NIOWebSocket

// MARK: - WebSocket tunnel

enum WebSocketBackendConnector {
  static func connect(frontend: any Channel, head: HTTPRequestHead) -> EventLoopFuture<Void> {
    guard let route = rackRoute(for: head.headers["host"].first),
      !route.socketPath.isEmpty || route.tcpPort > 0
    else {
      return frontend.eventLoop.makeFailedFuture(ProxyError.backendUnavailable)
    }

    let tunnel = WebSocketTunnel(eventLoop: frontend.eventLoop)
    let requestKey =
      head.headers["sec-websocket-key"].first
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
        do {
          try channel.pipeline.syncOperations.addHTTPClientHandlers(
            withClientUpgrade: upgradeConfig)
          try channel.pipeline.syncOperations.addHandler(requestHandler)
          return channel.eventLoop.makeSucceededVoidFuture()
        } catch {
          return channel.eventLoop.makeFailedFuture(error)
        }
      }

    let connectFuture =
      route.socketPath.isEmpty
      ? bootstrap.connect(host: "localhost", port: route.tcpPort)
      : bootstrap.connect(unixDomainSocketPath: route.socketPath)

    return connectFuture.flatMap { backend in
      tunnel.setFrontend(frontend)
      return frontend.pipeline.addHandler(WebSocketFrameRelay(tunnel: tunnel, side: .frontend)).map
      {
        _ = backend
      }
    }
  }
}

private final class WebSocketUpgradeRequestHandler: ChannelInboundHandler, RemovableChannelHandler,
  @unchecked Sendable
{
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
