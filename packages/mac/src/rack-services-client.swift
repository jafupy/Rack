import Foundation
import RackUI

@MainActor
final class RackServicesClient: RackRuntimeClient {
  func initialize() throws {
    try RackServices.check(rackServicesInit())
  }

  func services() throws -> [ServiceConfiguration] {
    let json = try RackServices.value(rackServicesSnapshotJSON())
    let snapshot = try JSONDecoder().decode(RegistrySnapshot.self, from: Data(json.utf8))
    return snapshot.services.map { ServiceConfiguration($0, proxyPort: snapshot.proxyPort) }
  }

  func startService(id: String) throws {
    try id.withCString { try RackServices.check(rackServicesStartService($0)) }
  }

  func stopService(id: String) throws {
    try id.withCString { try RackServices.check(rackServicesStopService($0)) }
  }

  func restartService(id: String) throws {
    try id.withCString { try RackServices.check(rackServicesRestartService($0)) }
  }

  func shutdown() {
    _ = RackServices.string(rackServicesShutdown())
  }

  func log(for id: String) -> String {
    (try? id.withCString { try RackServices.value(rackServicesLog($0)) }) ?? ""
  }

  func logFilePath(for id: String) -> String? { nil }
  func openInTerminal(id: String) {}
  func hooks() -> [HookSummary] { [] }
}

extension ServiceConfiguration {
  fileprivate init(_ service: ServiceSnapshot, proxyPort: UInt16?) {
    self.init(
      id: service.id,
      name: service.name,
      command: service.run,
      host: service.host,
      proxyPort: proxyPort,
      status: ServiceStatus(service.state)
    )
  }
}

extension ServiceStatus {
  fileprivate init(_ state: ServiceStateSnapshot) {
    switch state.kind {
    case "starting": self = .starting
    case "running": self = .running
    case "stopped": self = .stopped
    default: self = .failed
    }
  }
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
  let proxyPort: UInt16?
  let services: [ServiceSnapshot]

  enum CodingKeys: String, CodingKey {
    case proxyPort = "proxy_port"
    case services
  }
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

@_silgen_name("rack_services_init")
private func rackServicesInit() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_snapshot_json")
private func rackServicesSnapshotJSON() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_start_service")
private func rackServicesStartService(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_stop_service")
private func rackServicesStopService(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_restart_service")
private func rackServicesRestartService(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_log")
private func rackServicesLog(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_shutdown")
private func rackServicesShutdown() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_string_free")
private func rackServicesStringFree(_ value: UnsafeMutablePointer<CChar>)
