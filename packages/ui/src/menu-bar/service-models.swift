import Foundation

public struct ServiceConfiguration: Identifiable, Equatable {
  public let id: String
  public let name: String
  public let command: String
  public let arguments: String
  public let localURL: String
  public let status: ServiceStatus

  public init(
    id: String,
    name: String,
    command: String,
    arguments: String = "",
    host: String,
    proxyPort: UInt16? = nil,
    status: ServiceStatus
  ) {
    self.id = id
    self.name = name
    self.command = command
    self.arguments = arguments
    if let proxyPort {
      self.localURL = "http://\(host).localhost:\(proxyPort)"
    } else {
      self.localURL = "http://\(host).localhost"
    }
    self.status = status
  }
}

public enum ServiceStatus: Equatable {
  case stopped
  case starting
  case running
  case failed

  public var isRunning: Bool { self == .running }
  public var isActive: Bool { self == .starting || self == .running }
}

public enum RackProxy {
  public static let fallbackPort = 8080
}
