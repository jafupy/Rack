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

    var tint: String {
        switch self {
        case .stopped:
            return "secondary"
        case .starting:
            return "orange"
        case .running:
            return "green"
        case .failed:
            return "red"
        }
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

    var parsedArguments: [String] {
        arguments
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
    }

    /// Subdomain used for routing — custom if set, otherwise derived from name.
    var routeSubdomain: String {
        let raw = customDomain.isEmpty ? name : customDomain
        return raw.lowercased()
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: " ", with: "-")
    }

    /// The stable .localhost URL served by RackProxy.
    var localURL: String {
        "http://\(routeSubdomain).localhost:\(ProxyServer.defaultPort)"
    }
}

struct PersistedConfiguration: Codable {
    var servers: [ServerConfiguration]
}
