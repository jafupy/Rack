@preconcurrency import NIOCore
@preconcurrency import NIOHTTP1
import NIOPosix

extension HTTPProxyHandler {
  func isLoopback(_ host: String?) -> Bool {
    let info = RustProxy.hostInfo(for: host)
    guard info.isLoopbackCandidate, let routeName = info.routeName else { return false }
    return RouteRegistry.shared.route(for: routeName) == nil
  }

  /// Resolves the host on every attempt so the proxy picks up Rust route updates.
  func openBackend(
    context: ChannelHandlerContext, host: String, head: HTTPRequestHead, attempt: Int = 0
  ) {
    guard let route = resolve(host: host) else {
      sendError(
        context: context, status: .badGateway,
        body: "rack: no route for \(host)")
      return
    }

    // Not ready yet — socket path empty and no TCP port
    guard !route.socketPath.isEmpty || route.tcpPort > 0 else {
      if attempt < 120 && context.channel.isActive {
        let sendableContext = UnsafeSendableBox(value: context)
        context.eventLoop.scheduleTask(in: .milliseconds(500)) {
          self.openBackend(
            context: sendableContext.value, host: host, head: head, attempt: attempt + 1)
        }
      } else {
        sendError(
          context: context, status: .serviceUnavailable,
          body: "rack: server did not start within 60s")
      }
      return
    }

    let sendableContext = UnsafeSendableBox(value: context)
    let frontend = context.channel
    let bootstrap = ClientBootstrap(group: context.eventLoop)
      .channelInitializer { channel in
        channel.pipeline.addHTTPClientHandlers().flatMap {
          channel.pipeline.addHandler(
            BackendResponseHandler(frontend: frontend)
          )
        }
      }

    let connectFuture =
      route.socketPath.isEmpty
      ? bootstrap.connect(host: "localhost", port: route.tcpPort)
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
        if attempt < 120 && sendableContext.value.channel.isActive {
          sendableContext.value.eventLoop.scheduleTask(in: .milliseconds(500)) {
            self.openBackend(
              context: sendableContext.value,
              host: host,
              head: head,
              attempt: attempt + 1
            )
          }
        } else {
          self.sendError(
            context: sendableContext.value, status: .badGateway,
            body: "rack: backend not ready — is the server starting?")
        }
      }
    }
  }

  func writeBackend(
    _ backend: any Channel, _ part: HTTPClientRequestPart, flush: Bool = false
  ) {
    if flush {
      backend.writeAndFlush(part, promise: nil)
    } else {
      backend.write(part, promise: nil)
    }
  }
}
