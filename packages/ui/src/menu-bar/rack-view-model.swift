import Foundation

@MainActor
public final class RackViewModel: ObservableObject {
  @Published public private(set) var services: [ServiceConfiguration] = []
  @Published public private(set) var hooks: [HookSummary] = []

  private let runtime: RackRuntimeClient?
  private var refreshTask: Task<Void, Never>?

  public init(runtime: RackRuntimeClient? = nil) {
    self.runtime = runtime

    do {
      try runtime?.initialize()
      reloadServices()
      reloadHooks()
      startRefreshing()
    } catch {
      print("failed to initialize rack runtime: \(error)")
    }
  }

  deinit {
    refreshTask?.cancel()
  }

  public func status(for id: ServiceConfiguration.ID) -> ServiceStatus {
    services.first { $0.id == id }?.status ?? .stopped
  }

  public func start(id: ServiceConfiguration.ID) {
    do {
      try runtime?.startService(id: id)
      reloadServices()
    } catch {
      print("failed to start service \(id): \(error)")
    }
  }

  public func stop(id: ServiceConfiguration.ID) {
    do {
      try runtime?.stopService(id: id)
      reloadServices()
    } catch {
      print("failed to stop service \(id): \(error)")
    }
  }

  public func restart(id: ServiceConfiguration.ID) {
    do {
      try runtime?.restartService(id: id)
      reloadServices()
    } catch {
      print("failed to restart service \(id): \(error)")
    }
  }

  public func addService(_ service: ServiceDefinition) {
    do {
      try runtime?.addService(service)
      reloadServices()
    } catch {
      print("failed to add service \(service.id): \(error)")
    }
  }

  public func editService(id: ServiceConfiguration.ID, service: ServiceDefinition) {
    do {
      try runtime?.editService(id: id, service: service)
      reloadServices()
    } catch {
      print("failed to edit service \(id): \(error)")
    }
  }

  public func removeService(id: ServiceConfiguration.ID) {
    do {
      try runtime?.removeService(id: id)
      reloadServices()
    } catch {
      print("failed to remove service \(id): \(error)")
    }
  }

  public func stopAll() {
    for service in services where status(for: service.id).isActive {
      stop(id: service.id)
    }
  }

  public func log(for id: ServiceConfiguration.ID) -> String {
    runtime?.log(for: id) ?? ""
  }

  public func logFilePath(for id: ServiceConfiguration.ID) -> String? {
    runtime?.logFilePath(for: id)
  }

  public func openInTerminal(id: ServiceConfiguration.ID) {
    runtime?.openInTerminal(id: id)
  }

  public func configPath() -> String? {
    runtime?.configPath()
  }

  public func terminalName() -> String {
    runtime?.terminalName() ?? "Ghostty"
  }

  public func setTerminalName(_ terminal: String) throws {
    try runtime?.setTerminalName(terminal)
  }

  public func reloadHooks() {
    hooks = runtime?.hooks() ?? []
  }

  private func startRefreshing() {
    refreshTask = Task { [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: .milliseconds(500))
        self?.reloadServices()
      }
    }
  }

  private func reloadServices() {
    do {
      services = try runtime?.services() ?? []
    } catch {
      print("failed to refresh rack services: \(error)")
    }
  }
}

@MainActor
public protocol RackRuntimeClient: AnyObject {
  func initialize() throws
  func services() throws -> [ServiceConfiguration]
  func startService(id: String) throws
  func stopService(id: String) throws
  func restartService(id: String) throws
  func addService(_ service: ServiceDefinition) throws
  func editService(id: String, service: ServiceDefinition) throws
  func removeService(id: String) throws

  func shutdown()
  func log(for id: String) -> String
  func logFilePath(for id: String) -> String?
  func openInTerminal(id: String)
  func configPath() -> String?
  func terminalName() -> String
  func setTerminalName(_ terminal: String) throws
  func hooks() -> [HookSummary]
}

public struct HookSummary: Identifiable, Equatable {
  public let id: UUID
  public let name: String
  public let routes: [HookRoute]
  public let crons: [HookCron]
  public let errors: [String]

  public init(
    id: UUID = UUID(),
    name: String,
    routes: [HookRoute],
    crons: [HookCron],
    errors: [String]
  ) {
    self.id = id
    self.name = name
    self.routes = routes
    self.crons = crons
    self.errors = errors
  }
}

public struct HookRoute: Identifiable, Equatable {
  public let id: UUID
  public let method: String
  public let path: String

  public init(id: UUID = UUID(), method: String, path: String) {
    self.id = id
    self.method = method
    self.path = path
  }
}

public struct HookCron: Identifiable, Equatable {
  public let id: UUID
  public let schedule: String
  public let hook: String

  public init(id: UUID = UUID(), schedule: String, hook: String) {
    self.id = id
    self.schedule = schedule
    self.hook = hook
  }
}
