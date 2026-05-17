import Foundation

enum ServerStatus: Equatable {
    case stopped
    case starting
    case running(pid: Int32)
    case failed(message: String)

    var label: String {
        switch self {
        case .stopped:
            return "Stopped"
        case .starting:
            return "Starting"
        case .running(let pid):
            return "Running (\(pid))"
        case .failed(let message):
            return "Failed: \(message)"
        }
    }

    var isRunning: Bool {
        if case .running = self {
            return true
        }
        return false
    }

}

struct ServerConfiguration: Codable, Identifiable, Equatable {
    struct EnvironmentVariable: Codable, Identifiable, Equatable {
        var id: UUID = UUID()
        var key: String = ""
        var value: String = ""
    }

    var id: UUID = UUID()
    var name: String = "New Server"
    var command: String = ""
    var arguments: String = ""
    var workingDirectory: String = ""
    var autoStart: Bool = false
    var customDomain: String = ""
    var environment: [EnvironmentVariable] = []
    /// Explicit port the dev server listens on. When set the proxy routes directly to this port
    /// and skips PORT injection. When nil a free port is allocated and injected via PORT / portFlag.
    var port: Int? = nil
    /// CLI flag to pass the port number to servers that ignore the PORT env var (e.g. "--port", "-p").
    var portFlag: String? = nil

    var parsedArguments: [String] {
        arguments
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
    }

    /// Subdomain used for routing — custom if set, otherwise derived from name.
    var routeSubdomain: String {
        let raw = customDomain.isEmpty ? name : customDomain
        let trimmed = raw.lowercased()
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: " ", with: "-")
        return trimmed.hasSuffix(".localhost")
            ? String(trimmed.dropLast(".localhost".count))
            : trimmed
    }

    /// The .localhost URL served by the proxy. Omits the port if port-80 forwarding is active.
    var localURL: String {
        if UserDefaults.standard.bool(forKey: "standardPortsEnabled") {
            return "http://\(routeSubdomain).localhost"
        }
        return "http://\(routeSubdomain).localhost:\(ProxyServer.boundPort)"
    }
}

struct PersistedConfiguration: Codable {
    var servers: [ServerConfiguration]
}
