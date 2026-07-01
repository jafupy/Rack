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
    do {
      try runtime?.reloadHooks()
      hooks = runtime?.hooks() ?? []
    } catch {
      print("failed to reload hooks: \(error)")
    }
  }

  public func removeHook(name: String) {
    do {
      try runtime?.removeHook(name: name)
      hooks = runtime?.hooks() ?? []
    } catch {
      print("failed to remove hook \(name): \(error)")
    }
  }

  public func openHookDirectory(name: String) {
    runtime?.openHookDirectory(name: name)
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
