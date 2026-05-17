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

/// Thread-safe registry mapping server names to their TCP ports.
/// Uses NSLock so the proxy handler can look up routes synchronously from any thread.
final class RouteRegistry: @unchecked Sendable {
    private var routes: [String: Route] = [:]
    private let lock = NSLock()
    private let storageURL: URL

    static let shared = RouteRegistry()

    private init() {
        let dir = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".config/rack")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        storageURL = dir.appending(path: "routes.json")
        routes = (try? Self.load(from: storageURL)) ?? [:]
    }

    func register(_ route: Route) {
        lock.withLock { routes[route.name] = route }
        try? persist()
    }

    func updatePort(name: String, tcpPort: Int) {
        lock.withLock {
            guard let existing = routes[name] else { return }
            routes[name] = Route(name: existing.name, socketPath: existing.socketPath,
                                 tcpPort: tcpPort, workingDirectory: existing.workingDirectory,
                                 addedAt: existing.addedAt)
        }
        try? persist()
    }

    func updateSocketPath(name: String, socketPath: String) {
        lock.withLock {
            guard let existing = routes[name] else { return }
            routes[name] = Route(name: existing.name, socketPath: socketPath,
                                 tcpPort: existing.tcpPort, workingDirectory: existing.workingDirectory,
                                 addedAt: existing.addedAt)
        }
        try? persist()
    }

    func unregister(name: String) {
        lock.withLock { routes.removeValue(forKey: name) }
        try? persist()
    }

    /// Synchronous lookup — safe to call from NIO event loops.
    func route(for name: String) -> Route? {
        lock.withLock {
            // Exact match
            if let r = routes[name] { return r }
            // Subdomain fallback: fix-auth.myapp -> myapp
            let parts = name.split(separator: ".")
            guard parts.count > 1 else { return nil }
            let base = parts.dropFirst().joined(separator: ".")
            return routes[base]
        }
    }

    func allRoutes() -> [Route] {
        lock.withLock { Array(routes.values) }
    }

    func clearAll() {
        lock.withLock { routes = [:] }
        try? persist()
    }

    private func persist() throws {
        let snapshot = lock.withLock { routes }
        let data = try JSONEncoder().encode(snapshot)
        try data.write(to: storageURL, options: .atomic)
    }

    private static func load(from url: URL) throws -> [String: Route] {
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode([String: Route].self, from: data)
    }
}
