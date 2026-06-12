import Foundation

struct Route: Codable, Sendable {
  let name: String
  /// Unix socket path created by rack-bridge once the server is listening.
  /// Empty until the server is ready. Preferred over tcpPort when non-empty.
  var socketPath: String
  /// TCP port used when rack-bridge is unavailable. 0 until ready.
  var tcpPort: Int
  let workingDirectory: String
  let addedAt: Date
}

/// Compatibility facade over the Rust-owned route registry.
final class RouteRegistry: @unchecked Sendable {
  static let shared = RouteRegistry()

  private let encoder = JSONEncoder()
  private let decoder = JSONDecoder()

  private init() {}

  func register(_ route: Route) {
    _ = command(type: "routes.register", payload: route)
  }

  func updatePort(name: String, tcpPort: Int) {
    _ = command(type: "routes.updatePort", payload: RoutePortUpdate(name: name, tcpPort: tcpPort))
  }

  func updateSocketPath(name: String, socketPath: String) {
    _ = command(
      type: "routes.updateSocketPath",
      payload: RouteSocketUpdate(name: name, socketPath: socketPath))
  }

  func unregister(name: String) {
    _ = command(type: "routes.unregister", payload: RouteName(name: name))
  }

  /// Synchronous lookup; safe to call from NIO event loops.
  func route(for name: String) -> Route? {
    guard let data = command(type: "routes.resolve", payload: RouteName(name: name)) else {
      return nil
    }
    return try? decoder.decode(RouteReply.self, from: data).payload
  }

  func allRoutes() -> [Route] {
    guard let data = command(type: "routes.list", payload: EmptyPayload()) else { return [] }
    return (try? decoder.decode(RouteListReply.self, from: data).payload) ?? []
  }

  func clearAll() {
    _ = command(type: "routes.clear", payload: EmptyPayload())
  }

  private func command<Payload: Encodable>(type: String, payload: Payload) -> Data? {
    let command = CoreCommand(type: type, payload: payload)
    guard let data = try? encoder.encode(command),
      let json = String(data: data, encoding: .utf8),
      let response = RackCore.commandSync(json)
    else { return nil }
    return response.data(using: .utf8)
  }

  private struct CoreCommand<Payload: Encodable>: Encodable {
    var type: String
    var payload: Payload
  }

  private struct RouteName: Encodable {
    var name: String
  }

  private struct RoutePortUpdate: Encodable {
    var name: String
    var tcpPort: Int
  }

  private struct RouteSocketUpdate: Encodable {
    var name: String
    var socketPath: String
  }

  private struct EmptyPayload: Encodable {}

  private struct RouteReply: Decodable {
    var payload: Route?
  }

  private struct RouteListReply: Decodable {
    var payload: [Route]
  }
}
