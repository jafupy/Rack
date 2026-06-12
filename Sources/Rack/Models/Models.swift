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

    /// Subdomain used for routing. Rust owns the routing rules; Swift only displays the result.
    var routeSubdomain: String {
        routeInfo?.routeSubdomain ?? ""
    }

    /// The .localhost URL served by the proxy. Rust owns the URL rules; Swift only displays the result.
    var localURL: String {
        routeInfo?.localURL ?? ""
    }

    private var routeInfo: CoreServerRouteInfoReply.Payload? {
        let context = CoreServerRouteInfoContext(
            boundPort: ProxyServer.boundPort,
            standardPortsEnabled: UserDefaults.standard.bool(forKey: "standardPortsEnabled")
        )
        let payload = CoreServerRouteInfoRequest(config: self, context: context)
        let command = CoreServerRouteInfoCommand(type: "server.routeInfo", payload: payload)

        guard let data = try? JSONEncoder().encode(command),
            let json = String(data: data, encoding: .utf8),
            let response = RackCore.commandSync(json),
            let responseData = response.data(using: .utf8)
        else { return nil }

        return try? JSONDecoder().decode(CoreServerRouteInfoReply.self, from: responseData).payload
    }
}

struct PersistedConfiguration: Codable {
    var servers: [ServerConfiguration]
}

private struct CoreServerRouteInfoCommand: Encodable {
    var type: String
    var payload: CoreServerRouteInfoRequest
}

private struct CoreServerRouteInfoRequest: Encodable {
    var config: ServerConfiguration
    var context: CoreServerRouteInfoContext
}

private struct CoreServerRouteInfoContext: Encodable {
    var boundPort: Int
    var standardPortsEnabled: Bool
}

private struct CoreServerRouteInfoReply: Decodable {
    struct Payload: Decodable {
        var routeSubdomain: String
        var localURL: String
    }

    var payload: Payload
}
