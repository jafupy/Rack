import Foundation
import RackUI

@MainActor
final class RackServicesClient: RackRuntimeClient {
  func initialize() throws {
    try RackServicesABI.validate()
    try RackBridge.check(rackServicesInit())
  }

  func services() throws -> [ServiceConfiguration] {
    var snapshotPointer: UnsafeMutablePointer<RackServicesSnapshot>?
    try RackBridge.check(rackServicesSnapshot(&snapshotPointer))
    guard let snapshotPointer else { throw RackBridgeError.message("null services snapshot") }
    defer { rackServicesSnapshotFree(snapshotPointer) }

    let snapshot = snapshotPointer.pointee
    guard snapshot.abiVersion == RackABIVersion else {
      throw RackBridgeError.message("unsupported snapshot ABI version \(snapshot.abiVersion)")
    }

    let proxyPort = snapshot.hasProxyPort == 1 ? snapshot.proxyPort : nil
    let services = UnsafeBufferPointer(
      start: snapshot.services,
      count: snapshot.servicesLen
    )

    return services.map { ServiceConfiguration($0, proxyPort: proxyPort) }
  }

  func startService(id: String) throws {
    try id.withCString { try RackBridge.check(rackServicesStartService($0)) }
  }

  func stopService(id: String) throws {
    try id.withCString { try RackBridge.check(rackServicesStopService($0)) }
  }

  func shutdown() {
    RackBridge.discard(rackServicesShutdown())
  }

  func log(for id: String) -> String {
    (try? id.withCString { try RackBridge.value(rackServicesLog($0)) }) ?? ""
  }

  func logFilePath(for id: String) -> String? {
    try? id.withCString { try RackBridge.value(rackServicesLogPath($0)) }
  }

  func openInTerminal(id: String) {
    guard let path = logFilePath(for: id) else { return }
    openLogInTerminal(path: path, id: id)
  }

  func hooks() -> [HookSummary] {
    do {
      let json = try RackBridge.value(rackServicesHooksJson())
      return try JSONDecoder()
        .decode([RackHookSummaryPayload].self, from: Data(json.utf8))
        .map(HookSummary.init)
    } catch {
      print("failed to load hooks: \(error)")
      return []
    }
  }
}

extension ServiceConfiguration {
  fileprivate init(_ service: RackServicesServiceSnapshot, proxyPort: UInt16?) {
    self.init(
      id: RackBridge.string(service.id),
      name: RackBridge.string(service.name),
      command: RackBridge.string(service.run),
      host: RackBridge.string(service.host),
      proxyPort: proxyPort,
      status: ServiceStatus(service)
    )
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
