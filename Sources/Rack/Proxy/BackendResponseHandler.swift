@preconcurrency import NIOCore
@preconcurrency import NIOHTTP1

// MARK: - Backend response relay (HTTP mode)

final class BackendResponseHandler: ChannelInboundHandler, @unchecked Sendable {
  typealias InboundIn = HTTPClientResponsePart
  typealias OutboundOut = HTTPServerResponsePart

  private let frontend: any Channel

  init(frontend: any Channel) {
    self.frontend = frontend
  }

  func channelRead(context: ChannelHandlerContext, data: NIOAny) {
    switch unwrapInboundIn(data) {
    case .head(let head):
      let responseHead = HTTPResponseHead(
        version: head.version, status: head.status,
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
