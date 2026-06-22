import Foundation
import SwiftUI

@_silgen_name("rack_services_init")
private func rackServicesInit() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_snapshot_json")
private func rackServicesSnapshotJSON() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_start_service")
private func rackServicesStartService(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_stop_service")
private func rackServicesStopService(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_shutdown")
private func rackServicesShutdown() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_string_free")
private func rackServicesStringFree(_ value: UnsafeMutablePointer<CChar>?)

@MainActor
public final class ServerStore: ObservableObject {
  @Published var servers: [ServerConfiguration] = []
  @Published var functions: [FunctionSummary] = []

  private var logs: [ServerConfiguration.ID: String] = [:]
  private var logFilePaths: [ServerConfiguration.ID: String] = [:]
  private var refreshTask: Task<Void, Never>?

  public init() {
    do {
      try RackServices.check(rackServicesInit())
      reloadServices()
      startRefreshing()
    } catch {
      print("failed to initialize rack services: \(error)")
    }
  }

  deinit {
    refreshTask?.cancel()
    _ = RackServices.string(rackServicesShutdown())
  }

  func status(for id: ServerConfiguration.ID) -> ServerStatus {
    servers.first { $0.id == id }?.status ?? .stopped
  }

  func startServer(id: ServerConfiguration.ID) {
    do {
      try id.withCString { try RackServices.check(rackServicesStartService($0)) }
      reloadServices()
    } catch {
      print("failed to start service \(id): \(error)")
    }
  }

  func stopServer(id: ServerConfiguration.ID) {
    do {
      try id.withCString { try RackServices.check(rackServicesStopService($0)) }
      reloadServices()
    } catch {
      print("failed to stop service \(id): \(error)")
    }
  }

  func stopAllServers() {
    for server in servers
    where status(for: server.id).isRunning || status(for: server.id) == .starting {
      stopServer(id: server.id)
    }
  }

  func log(for id: ServerConfiguration.ID) -> String {
    logs[id, default: ""]
  }

  func logFilePath(for id: ServerConfiguration.ID) -> String? {
    logFilePaths[id]
  }

  func openInTerminal(id: ServerConfiguration.ID) {}

  func reloadFunctions() {}

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
      let json = try RackServices.value(rackServicesSnapshotJSON())
      let snapshot = try JSONDecoder().decode(RegistrySnapshot.self, from: Data(json.utf8))
      servers = snapshot.services.map(ServerConfiguration.init)
    } catch {
      print("failed to refresh rack services: \(error)")
    }
  }

  struct FunctionSummary: Identifiable {
    let id = UUID()
    let name: String
    let routes: [FunctionRoute]
    let crons: [FunctionCron]
    let errors: [String]
  }

  struct FunctionRoute: Identifiable {
    let id = UUID()
    let method: String
    let path: String
  }

  struct FunctionCron: Identifiable {
    let id = UUID()
    let schedule: String
    let function: String
  }
}

struct ServerConfiguration: Identifiable {
  let id: String
  let name: String
  let command: String
  let arguments: String
  let localURL: String
  let status: ServerStatus

  fileprivate init(_ service: ServiceSnapshot) {
    self.id = service.id
    self.name = service.name
    self.command = service.run
    self.arguments = ""
    self.localURL = "http://\(service.host).local"
    self.status = ServerStatus(service.state)
  }
}

enum ServerStatus: Equatable {
  case stopped
  case starting
  case running
  case failed

  var isRunning: Bool { self == .running }

  fileprivate init(_ state: ServiceStateSnapshot) {
    switch state.kind {
    case "starting": self = .starting
    case "running": self = .running
    case "stopped": self = .stopped
    default: self = .failed
    }
  }
}

enum ProxyServer {
  static let boundPort = 8080
}

private enum RackServices {
  static func check(_ pointer: UnsafeMutablePointer<CChar>?) throws {
    let response = string(pointer)
    if response.hasPrefix("ERROR:") {
      throw RackServicesError.message(String(response.dropFirst("ERROR:".count)))
    }
  }

  static func value(_ pointer: UnsafeMutablePointer<CChar>?) throws -> String {
    let response = string(pointer)
    if response.hasPrefix("ERROR:") {
      throw RackServicesError.message(String(response.dropFirst("ERROR:".count)))
    }
    return response
  }

  static func string(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
    guard let pointer else { return "ERROR:null ffi response" }
    defer { rackServicesStringFree(pointer) }
    return String(cString: pointer)
  }
}

private enum RackServicesError: Error, CustomStringConvertible {
  case message(String)

  var description: String {
    switch self {
    case .message(let message): message
    }
  }
}

private struct RegistrySnapshot: Decodable {
  let services: [ServiceSnapshot]
}

private struct ServiceSnapshot: Decodable {
  let id: String
  let name: String
  let host: String
  let run: String
  let workingDir: String
  let autoStart: Bool
  let state: ServiceStateSnapshot

  enum CodingKeys: String, CodingKey {
    case id
    case name
    case host
    case run
    case workingDir = "working_dir"
    case autoStart = "auto_start"
    case state
  }
}

private struct ServiceStateSnapshot: Decodable {
  let kind: String
  let pid: Int?
  let pgid: Int?
  let ports: [UInt16]?
}
