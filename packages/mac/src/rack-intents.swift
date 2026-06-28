import AppIntents
import Foundation
import RackUI

@MainActor
enum RackIntentBridge {
  static var model: RackViewModel?
  static let launchAtLogin = LaunchAtLoginController()

  static func services() throws -> [ServiceConfiguration] {
    guard let model else { throw RackIntentError.appUnavailable }
    return model.services
  }

  static func service(id: String) throws -> ServiceConfiguration? {
    try services().first { $0.id == id }
  }
}

enum RackIntentError: LocalizedError {
  case appUnavailable
  case serviceNotFound

  var errorDescription: String? {
    switch self {
    case .appUnavailable: "Rack is not ready yet."
    case .serviceNotFound: "That Rack service no longer exists."
    }
  }
}

struct RackServiceEntity: AppEntity, Identifiable {
  static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Rack Service")
  static let defaultQuery = RackServiceEntityQuery()

  let id: String
  let name: String
  let command: String
  let localURL: String

  init(_ service: ServiceConfiguration) {
    id = service.id
    name = service.name.isEmpty ? "Unnamed Service" : service.name
    command = [service.command, service.arguments].filter { !$0.isEmpty }.joined(separator: " ")
    localURL = service.localURL
  }

  var displayRepresentation: DisplayRepresentation {
    DisplayRepresentation(
      title: "\(name)",
      subtitle: command.isEmpty ? "\(localURL)" : "\(command)"
    )
  }
}

struct RackServiceEntityQuery: EntityStringQuery, EnumerableEntityQuery {
  @MainActor
  func entities(for identifiers: [RackServiceEntity.ID]) async throws -> [RackServiceEntity] {
    try identifiers.compactMap { try RackIntentBridge.service(id: $0).map(RackServiceEntity.init) }
  }

  @MainActor
  func entities(matching string: String) async throws -> [RackServiceEntity] {
    let query = string.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    guard !query.isEmpty else { return try await allEntities() }
    return try RackIntentBridge.services()
      .filter { service in
        service.name.lowercased().contains(query)
          || service.command.lowercased().contains(query)
          || service.localURL.lowercased().contains(query)
      }
      .map(RackServiceEntity.init)
  }

  @MainActor
  func suggestedEntities() async throws -> [RackServiceEntity] {
    try await allEntities()
  }

  @MainActor
  func allEntities() async throws -> [RackServiceEntity] {
    try RackIntentBridge.services().map(RackServiceEntity.init)
  }
}

struct StartRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Start Rack Service"
  static let description = IntentDescription("Start one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.start(id: service.id)
    return .result(dialog: "Starting \(service.name)")
  }
}

struct StopRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Stop Rack Service"
  static let description = IntentDescription("Stop one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.stop(id: service.id)
    return .result(dialog: "Stopped \(service.name)")
  }
}

struct RestartRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Restart Rack Service"
  static let description = IntentDescription("Restart one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.restart(id: service.id)
    return .result(dialog: "Restarting \(service.name)")
  }
}

struct StopAllRackServicesIntent: AppIntent {
  static let title: LocalizedStringResource = "Stop All Rack Services"
  static let description = IntentDescription("Stop every active Rack service.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    model.stopAll()
    return .result(dialog: "Stopped all Rack services")
  }
}

struct ReloadRackHooksIntent: AppIntent {
  static let title: LocalizedStringResource = "Reload Rack Hooks"
  static let description = IntentDescription("Refresh Rack hook summaries.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    model.reloadHooks()
    return .result(dialog: "Reloaded Rack hooks")
  }
}

struct EnableRackLaunchAtLoginIntent: AppIntent {
  static let title: LocalizedStringResource = "Enable Rack Launch at Login"
  static let description = IntentDescription("Register Rack as a login item.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    RackIntentBridge.launchAtLogin.setEnabled(true)
    return .result(dialog: "Enabled Rack launch at login")
  }
}

struct DisableRackLaunchAtLoginIntent: AppIntent {
  static let title: LocalizedStringResource = "Disable Rack Launch at Login"
  static let description = IntentDescription("Remove Rack from login items.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    RackIntentBridge.launchAtLogin.setEnabled(false)
    return .result(dialog: "Disabled Rack launch at login")
  }
}

struct RackShortcuts: AppShortcutsProvider {
  static let shortcutTileColor: ShortcutTileColor = .blue

  static var appShortcuts: [AppShortcut] {
    AppShortcut(
      intent: StartRackServiceIntent(),
      phrases: ["Start \(\.$service) in \(.applicationName)"],
      shortTitle: "Start Service",
      systemImageName: "play.fill"
    )
    AppShortcut(
      intent: StopRackServiceIntent(),
      phrases: ["Stop \(\.$service) in \(.applicationName)"],
      shortTitle: "Stop Service",
      systemImageName: "stop.fill"
    )
    AppShortcut(
      intent: RestartRackServiceIntent(),
      phrases: ["Restart \(\.$service) in \(.applicationName)"],
      shortTitle: "Restart Service",
      systemImageName: "arrow.clockwise"
    )
    AppShortcut(
      intent: StopAllRackServicesIntent(),
      phrases: ["Stop all services in \(.applicationName)"],
      shortTitle: "Stop All",
      systemImageName: "stop.circle"
    )
    AppShortcut(
      intent: ReloadRackHooksIntent(),
      phrases: ["Reload hooks in \(.applicationName)"],
      shortTitle: "Reload Hooks",
      systemImageName: "point.3.connected.trianglepath.dotted"
    )
  }
}
