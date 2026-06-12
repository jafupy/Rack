import Dispatch
import Foundation
@preconcurrency import NIOCore
@preconcurrency import NIOHTTP1
import NIOPosix

// MARK: - Proxy handler

final class HTTPProxyHandler: ChannelInboundHandler, @unchecked Sendable {
  typealias InboundIn = HTTPServerRequestPart
  typealias OutboundOut = HTTPServerResponsePart

  private var pendingHead: HTTPRequestHead?
  var bodyBuffer: ByteBuffer?
  var pendingEnd: HTTPHeaders?
  var backendChannel: (any Channel)?
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
        sendError(
          context: context, status: .badGateway,
          body: "rack: no route for \(head.headers["host"].first ?? "unknown")")
        return
      }

      if isLoopback(head.headers["host"].first) {
        sendError(
          context: context, status: .custom(code: 508, reasonPhrase: "Loop Detected"),
          body:
            "rack: proxy loop detected — check your Vite/webpack proxy config and set changeOrigin: true"
        )
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

  func resolve(host: String?) -> Route? {
    rackRoute(for: host)
  }

  private func handleRackLocal(context: ChannelHandlerContext, head: HTTPRequestHead) {
    let body = bodyBuffer.map { String(buffer: $0) } ?? ""
    guard
      let request = RustProxy.rackLocalRequest(
        method: head.method.rawValue,
        uri: head.uri,
        headers: head.headers,
        body: body
      )
    else {
      sendError(
        context: context, status: .internalServerError, body: "rack: function dispatch failed")
      return
    }

    switch request["kind"] as? String {
    case "root":
      sendPlainResponse(
        context: context,
        status: .ok,
        body: "Rack.\n",
        headers: ["content-type": "text/plain"]
      )
      return

    case "reserved":
      sendError(context: context, status: .notFound, body: "rack: reserved path")
      return

    case "function":
      break

    default:
      sendError(
        context: context, status: .internalServerError, body: "rack: function dispatch failed")
      return
    }

    guard let commandObject = request["command"] as? [String: Any],
      let commandData = try? JSONSerialization.data(withJSONObject: commandObject),
      let command = String(data: commandData, encoding: .utf8)
    else {
      sendError(
        context: context, status: .internalServerError, body: "rack: function dispatch failed")
      return
    }

    let responseContext = RackLocalResponseContext(context: context)
    DispatchQueue.global(qos: .userInitiated).async {
      let result = RackLocalFunctionThreadLimiter.shared.run {
        dispatchRackLocalFunction(command)
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

  private func appendToBodyBuffer(_ buffer: ByteBuffer) {
    if bodyBuffer == nil {
      bodyBuffer = buffer
    } else {
      bodyBuffer?.writeImmutableBuffer(buffer)
    }
  }

  func sendError(context: ChannelHandlerContext, status: HTTPResponseStatus, body: String) {
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
    let sendableContext = UnsafeSendableBox(value: context)
    context.writeAndFlush(wrapOutboundOut(.end(nil))).whenComplete { _ in
      sendableContext.value.close(promise: nil)
    }
  }
}
