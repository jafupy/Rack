import AppIntents
import AppKit
import Foundation

enum RackIntentError: LocalizedError {
    case appUnavailable
    case serverNotFound

    var errorDescription: String? {
        switch self {
        case .appUnavailable:
            return "Rack is not ready yet."
        case .serverNotFound:
            return "That Rack server no longer exists."
        }
    }
}

@MainActor
enum RackIntentBridge {
    static var store: ServerStore? {
        NSApplication.shared.delegate.flatMap { $0 as? AppDelegate }?.store
    }

    static func configuredServers() throws -> [ServerConfiguration] {
        guard let store else { throw RackIntentError.appUnavailable }
        store.reloadServers()
        return store.servers
    }

    static func entity(for id: UUID) throws -> RackServerEntity? {
        try configuredServers()
            .first { $0.id == id }
            .map(RackServerEntity.init)
    }

    static func entity(matching string: String) throws -> [RackServerEntity] {
        let query = string.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else {
            return try configuredServers().map(RackServerEntity.init)
        }
        return try configuredServers()
            .filter { server in
                server.name.lowercased().contains(query)
                    || server.command.lowercased().contains(query)
                    || server.workingDirectory.lowercased().contains(query)
                    || server.routeSubdomain.lowercased().contains(query)
            }
            .map(RackServerEntity.init)
    }
}

struct RackServerEntity: AppEntity, Identifiable {
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Rack Server")
    static let defaultQuery = RackServerEntityQuery()

    let id: UUID
    let name: String
    let command: String
    let localURL: String

    init(id: UUID, name: String, command: String, localURL: String) {
        self.id = id
        self.name = name
        self.command = command
        self.localURL = localURL
    }

    init(_ server: ServerConfiguration) {
        self.init(
            id: server.id,
            name: server.name.isEmpty ? "Unnamed Server" : server.name,
            command: [server.command, server.arguments].filter { !$0.isEmpty }.joined(separator: " "),
            localURL: server.localURL
        )
    }

    var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(
            title: "\(name)",
            subtitle: command.isEmpty ? "\(localURL)" : "\(command)"
        )
    }
}

struct RackServerEntityQuery: EntityStringQuery, EnumerableEntityQuery {
    @MainActor
    func entities(for identifiers: [RackServerEntity.ID]) async throws -> [RackServerEntity] {
        try identifiers.compactMap { try RackIntentBridge.entity(for: $0) }
    }

    @MainActor
    func entities(matching string: String) async throws -> [RackServerEntity] {
        try RackIntentBridge.entity(matching: string)
    }

    @MainActor
    func suggestedEntities() async throws -> [RackServerEntity] {
        try RackIntentBridge.configuredServers().map(RackServerEntity.init)
    }

    @MainActor
    func allEntities() async throws -> [RackServerEntity] {
        try RackIntentBridge.configuredServers().map(RackServerEntity.init)
    }
}

struct StartRackServerIntent: AppIntent {
    static let title: LocalizedStringResource = "Start Rack Server"
    static let description = IntentDescription("Start one configured Rack server.")
    static let openAppWhenRun = true

    @Parameter(title: "Server")
    var server: RackServerEntity

    @MainActor
    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let store = RackIntentBridge.store else { throw RackIntentError.appUnavailable }
        store.reloadServers()
        guard store.servers.contains(where: { $0.id == server.id }) else {
            throw RackIntentError.serverNotFound
        }
        store.startServer(id: server.id)
        return .result(dialog: "Starting \(server.name)")
    }
}

struct StopRackServerIntent: AppIntent {
    static let title: LocalizedStringResource = "Stop Rack Server"
    static let description = IntentDescription("Stop one configured Rack server.")
    static let openAppWhenRun = true

    @Parameter(title: "Server")
    var server: RackServerEntity

    @MainActor
    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let store = RackIntentBridge.store else { throw RackIntentError.appUnavailable }
        guard store.servers.contains(where: { $0.id == server.id }) else {
            throw RackIntentError.serverNotFound
        }
        store.stopServer(id: server.id)
        return .result(dialog: "Stopped \(server.name)")
    }
}

struct RestartRackServerIntent: AppIntent {
    static let title: LocalizedStringResource = "Restart Rack Server"
    static let description = IntentDescription("Restart one configured Rack server.")
    static let openAppWhenRun = true

    @Parameter(title: "Server")
    var server: RackServerEntity

    @MainActor
    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let store = RackIntentBridge.store else { throw RackIntentError.appUnavailable }
        store.reloadServers()
        guard store.servers.contains(where: { $0.id == server.id }) else {
            throw RackIntentError.serverNotFound
        }
        store.restartServer(id: server.id)
        return .result(dialog: "Restarting \(server.name)")
    }
}

struct StopAllRackServersIntent: AppIntent {
    static let title: LocalizedStringResource = "Stop All Rack Servers"
    static let description = IntentDescription("Stop every configured Rack server.")
    static let openAppWhenRun = true

    @MainActor
    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let store = RackIntentBridge.store else { throw RackIntentError.appUnavailable }
        store.stopAllServers()
        return .result(dialog: "Stopped all Rack servers")
    }
}

struct ReloadRackFunctionsIntent: AppIntent {
    static let title: LocalizedStringResource = "Reload Rack Functions"
    static let description = IntentDescription("Refresh local Rack function packages.")
    static let openAppWhenRun = true

    @MainActor
    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let store = RackIntentBridge.store else { throw RackIntentError.appUnavailable }
        store.reloadFunctions()
        return .result(dialog: "Reloaded Rack functions")
    }
}

struct RackShortcuts: AppShortcutsProvider {
    static let shortcutTileColor: ShortcutTileColor = .blue

    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: StartRackServerIntent(),
            phrases: [
                "Start \(\.$server) in \(.applicationName)",
                "Start \(\.$server) with \(.applicationName)"
            ],
            shortTitle: "Start Server",
            systemImageName: "play.fill"
        )
        AppShortcut(
            intent: StopRackServerIntent(),
            phrases: [
                "Stop \(\.$server) in \(.applicationName)",
                "Stop \(\.$server) with \(.applicationName)"
            ],
            shortTitle: "Stop Server",
            systemImageName: "stop.fill"
        )
        AppShortcut(
            intent: RestartRackServerIntent(),
            phrases: [
                "Restart \(\.$server) in \(.applicationName)",
                "Restart \(\.$server) with \(.applicationName)"
            ],
            shortTitle: "Restart Server",
            systemImageName: "arrow.clockwise"
        )
        AppShortcut(
            intent: StopAllRackServersIntent(),
            phrases: [
                "Stop all servers in \(.applicationName)",
                "Stop all \(.applicationName) servers"
            ],
            shortTitle: "Stop All",
            systemImageName: "stop.circle"
        )
        AppShortcut(
            intent: ReloadRackFunctionsIntent(),
            phrases: [
                "Reload functions in \(.applicationName)",
                "Refresh \(.applicationName) functions"
            ],
            shortTitle: "Reload Functions",
            systemImageName: "function"
        )
    }
}
