import Foundation

let RackABIVersion: UInt32 = 1

enum RackBridgeError: Error, CustomStringConvertible {
  case message(String)

  var description: String {
    switch self {
    case .message(let message): message
    }
  }
}

enum RackBridge {
  static func check(_ status: RackServicesStatus) throws {
    defer { rackServicesStatusFree(status) }
    guard status.abiVersion == RackABIVersion else {
      throw RackBridgeError.message("unsupported Rack ABI version \(status.abiVersion)")
    }
    guard status.code == RackServicesStatusOk else {
      throw RackBridgeError.message(string(status.message))
    }
  }

  static func discard(_ status: RackServicesStatus) {
    rackServicesStatusFree(status)
  }

  static func value(_ pointer: UnsafeMutablePointer<CChar>?) throws -> String {
    let response = ownedString(pointer)
    if response.hasPrefix("ERROR:") {
      throw RackBridgeError.message(String(response.dropFirst("ERROR:".count)))
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
