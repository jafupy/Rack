@preconcurrency import NIOCore

func rackRoute(for host: String?) -> Route? {
  guard let name = RustProxy.hostInfo(for: host).routeName else { return nil }
  return RouteRegistry.shared.route(for: name)
}

func isRackLocalHost(_ host: String?) -> Bool {
  RustProxy.hostInfo(for: host).isRackLocalHost
}

enum ProxyError: Error {
  case backendUnavailable
}
