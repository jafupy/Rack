import Foundation
@preconcurrency import NIOCore

enum RackLocalFunctionError: Error {
  case invalidResponse
}

struct RackLocalFunctionResponse: Sendable {
  var statusCode: Int
  var headers: [String: String]
  var body: String
}

struct RackLocalResponseContext: @unchecked Sendable {
  var context: ChannelHandlerContext
}

final class RackLocalFunctionThreadLimiter: @unchecked Sendable {
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

func dispatchRackLocalFunction(_ command: String) -> Result<
  RackLocalFunctionResponse, RackLocalFunctionError
> {
  guard let responseJSON = RackCore.commandSync(command),
    let responseData = responseJSON.data(using: .utf8),
    let response = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any],
    let payload = response["payload"] as? [String: Any]
  else {
    return .failure(RackLocalFunctionError.invalidResponse)
  }

  return .success(
    RackLocalFunctionResponse(
      statusCode: payload["status"] as? Int ?? 500,
      headers: payload["headers"] as? [String: String] ?? ["content-type": "text/plain"],
      body: payload["body"] as? String ?? ""
    ))
}
