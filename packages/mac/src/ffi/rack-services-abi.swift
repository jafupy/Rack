import Foundation

let RackServicesStatusOk: UInt32 = 0
let RackServicesStateStopped: UInt32 = 0
let RackServicesStateStarting: UInt32 = 1
let RackServicesStateRunning: UInt32 = 2
let RackServicesStateFailed: UInt32 = 3

struct RackServicesStatus {
  let abiVersion: UInt32
  let code: UInt32
  let message: UnsafeMutablePointer<CChar>?
}

struct RackServicesSnapshot {
  let abiVersion: UInt32
  let hasProxyPort: UInt8
  let proxyPort: UInt16
  let servicesLen: Int
  let services: UnsafeMutablePointer<RackServicesServiceSnapshot>?
}

struct RackServicesServiceSnapshot {
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

enum RackServicesABI {
  static func validate() throws {
    try expect(rackServicesAbiVersion(), RackABIVersion, "services ABI version")
    try expect(
      rackServicesStatusSize(), MemoryLayout<RackServicesStatus>.size, "RackServicesStatus size")
    try expect(
      rackServicesSnapshotSize(), MemoryLayout<RackServicesSnapshot>.size,
      "RackServicesSnapshot size")
    try expect(
      rackServicesServiceSnapshotSize(),
      MemoryLayout<RackServicesServiceSnapshot>.size,
      "RackServicesServiceSnapshot size"
    )
  }

  private static func expect<T: BinaryInteger>(_ actual: T, _ expected: T, _ name: String) throws {
    guard actual == expected else {
      throw RackBridgeError.message("\(name) mismatch: rust=\(actual) swift=\(expected)")
    }
  }
}

@_silgen_name("rack_services_abi_version")
func rackServicesAbiVersion() -> UInt32

@_silgen_name("rack_services_status_size")
func rackServicesStatusSize() -> Int

@_silgen_name("rack_services_snapshot_size")
func rackServicesSnapshotSize() -> Int

@_silgen_name("rack_services_service_snapshot_size")
func rackServicesServiceSnapshotSize() -> Int

@_silgen_name("rack_services_init")
func rackServicesInit() -> RackServicesStatus

@_silgen_name("rack_services_snapshot")
func rackServicesSnapshot(
  _ out: UnsafeMutablePointer<UnsafeMutablePointer<RackServicesSnapshot>?>
) -> RackServicesStatus

@_silgen_name("rack_services_snapshot_free")
func rackServicesSnapshotFree(_ snapshot: UnsafeMutablePointer<RackServicesSnapshot>)

@_silgen_name("rack_services_start_service")
func rackServicesStartService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_stop_service")
func rackServicesStopService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_restart_service")
func rackServicesRestartService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_add_service_json")
func rackServicesAddServiceJson(_ serviceJson: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_edit_service_json")
func rackServicesEditServiceJson(
  _ id: UnsafePointer<CChar>, _ serviceJson: UnsafePointer<CChar>
) -> RackServicesStatus

@_silgen_name("rack_services_remove_service")
func rackServicesRemoveService(_ id: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_log")
func rackServicesLog(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_log_path")
func rackServicesLogPath(_ id: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_config_path")
func rackServicesConfigPath() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_terminal")
func rackServicesTerminal() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_set_terminal")
func rackServicesSetTerminal(_ terminal: UnsafePointer<CChar>) -> RackServicesStatus

@_silgen_name("rack_services_shutdown")
func rackServicesShutdown() -> RackServicesStatus

@_silgen_name("rack_services_hooks_json")
func rackServicesHooksJson() -> UnsafeMutablePointer<CChar>?

@_silgen_name("rack_services_status_free")
func rackServicesStatusFree(_ status: RackServicesStatus)

@_silgen_name("rack_services_string_free")
func rackServicesStringFree(_ value: UnsafeMutablePointer<CChar>)
