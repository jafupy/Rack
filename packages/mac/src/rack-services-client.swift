import Foundation
import RackUI

@MainActor
final class RackServicesClient: RackRuntimeClient {
  func initialize() throws {
    try RackServices.check(rackServicesInit())
  }

  func services() throws -> [ServiceConfiguration] {
    var snapshotPointer: UnsafeMutablePointer<RackServicesSnapshot>?
    try RackServices.check(rackServicesSnapshot(&snapshotPointer))
    guard let snapshotPointer else { throw RackServicesError.message("null services snapshot") }
    defer { rackServicesSnapshotFree(snapshotPointer) }

    let snapshot = snapshotPointer.pointee
    let proxyPort = snapshot.hasProxyPort == 1 ? snapshot.proxyPort : nil
    let services = UnsafeBufferPointer(
      start: snapshot.services,
      count: snapshot.servicesLen
    )

    return services.map { ServiceConfiguration($0, proxyPort: proxyPort) }
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
    RackServices.discard(rackServicesShutdown())
  }

  func log(for id: String) -> String {
    (try? id.withCString { try RackServices.value(rackServicesLog($0)) }) ?? ""
  }

  func logFilePath(for id: String) -> String? { nil }
  func openInTerminal(id: String) {}

  func hooks() -> [HookSummary] {
    do {
      let json = try RackServices.value(rackServicesHooksJson())
      return try JSONDecoder().decode([RackHookSummary].self, from: Data(json.utf8)).map(
        HookSummary.init)
    } catch {
      print("failed to load hooks: \(error)")
      return []
    }
  }
}

extension ServiceConfiguration {
  fileprivate init(_ service: RackServicesServiceSnapshot, proxyPort: UInt16?) {
    self.init(
      id: RackServices.string(service.id),
      name: RackServices.string(service.name),
      command: RackServices.string(service.run),
      host: RackServices.string(service.host),
      proxyPort: proxyPort,
      status: ServiceStatus(service)
    )
  }
}

extension HookSummary {
  fileprivate init(_ hook: RackHookSummary) {
    self.init(
      name: hook.name,
      routes: hook.routes.map(HookRoute.init),
      crons: hook.crons.map(HookCron.init),
      errors: hook.errors
    )
  }
}

extension HookRoute {
  fileprivate init(_ route: RackHookRouteSummary) {
    self.init(method: route.method, path: route.path)
  }
}

extension HookCron {
  fileprivate init(_ cron: RackHookCronSummary) {
    self.init(schedule: cron.schedule, hook: cron.hook)
  }
}

extension ServiceStatus {
  fileprivate init(_ service: RackServicesServiceSnapshot) {
    switch service.state {
    case RackServicesStateRunning: self = .running
    case RackServicesStateStarting: self = .starting
    case RackServicesStateStopped: self = .stopped
    default: self = .failed
    }
  }
}

private enum RackServices {
  static func check(_ status: RackServicesStatus) throws {
    defer { rackServicesStatusFree(status) }
    guard status.code == RackServicesStatusOk else {
      throw RackServicesError.message(string(status.message))
    }
  }

  static func discard(_ status: RackServicesStatus) {
    rackServicesStatusFree(status)
  }

  static func value(_ pointer: UnsafeMutablePointer<CChar>?) throws -> String {
    let response = ownedString(pointer)
    if response.hasPrefix("ERROR:") {
      throw RackServicesError.message(String(response.dropFirst("ERROR:".count)))
    }
    return response
  }

  static func ownedString(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
    guard let pointer else { return "ERROR:null ffi response" }
    defer { rackServicesStringFree(pointer) }
    return String(cString: pointer)
  }

  static func string(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
    guard let pointer else { return "" }
    return String(cString: pointer)
  }
}

private struct RackHookSummary: Decodable {
  let name: String
  let routes: [RackHookRouteSummary]
  let crons: [RackHookCronSummary]
  let errors: [String]
}

private struct RackHookRouteSummary: Decodable {
  let method: String
  let path: String
}

private struct RackHookCronSummary: Decodable {
  let schedule: String
  let hook: String
}

private enum RackServicesError: Error, CustomStringConvertible {
  case message(String)

  var description: String {
    switch self {
    case .message(let message): message
    }
  }
}

private let RackServicesStatusOk: UInt32 = 0
private let RackServicesStateStopped: UInt32 = 0
private let RackServicesStateStarting: UInt32 = 1
private let RackServicesStateRunning: UInt32 = 2

private struct RackServicesStatus {
  let abiVersion: UInt32
  let code: UInt32
  let message: UnsafeMutablePointer<CChar>?
}

private struct RackServicesSnapshot {
  let abiVersion: UInt32
  let hasProxyPort: UInt8
  let proxyPort: UInt16
  let servicesLen: Int
  let services: UnsafeMutablePointer<RackServicesServiceSnapshot>?
}

private struct RackServicesServiceSnapshot {
  let abiVersion: UInt32
  let state: UInt32
  let autoStart: UInt8
  let pid: Int32
  let pgid: Int32
  let id: UnsafeMutablePointer<CChar>?
  let name: UnsafeMutablePointer<CChar>?
  let host: UnsafeMutablePointer<CChar>?
  let run: UnsafeMutablePointer<CChar>?
  let workingDir: UnsafeMutablePointer<CChar>?
  let portsLen: Int
  let ports: UnsafeMutablePointer<UInt16>?
}

@_silgen_name("rack_services_init")
private func rackServicesInit() -> RackServicesStatus

@_silgen_name("rack_services_snapshot")
private func rackServicesSnapshot(
  _ out: UnsafeMutablePointer<UnsafeMutablePointer<RackServicesSnapshot>?>
) -> RackServicesStatus

@_silgen_name("rack_services_snapshot_free")
private func rackServicesSnapshotFree(_ snapshot: UnsafeMutablePointer<RackServicesSnapshot>)

@_silgen_name("rack_services_start_service")
private func rackServicesStartService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_stop_service")
private func rackServicesStopService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_restart_service")
private func rackServicesRestartService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_log")
private func rackServicesLog(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_shutdown")
private func rackServicesShutdown() -> RackServicesStatus

@_silgen_name("rack_services_hooks_json")
private func rackServicesHooksJson() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_status_free")
private func rackServicesStatusFree(_ status: RackServicesStatus)

@_silgen_name("rack_services_string_free")
private func rackServicesStringFree(_ value: UnsafeMutablePointer<CChar>)
