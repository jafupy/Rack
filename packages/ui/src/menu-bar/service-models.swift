import Foundation

public struct ServiceConfiguration: Identifiable, Equatable {
  public let id: String
  public let name: String
  public let command: String
  public let arguments: String
  public let host: String
  public let workingDir: String
  public let autoStart: Bool
  public let localURL: String
  public let status: ServiceStatus

  public init(
    id: String,
    name: String,
    command: String,
    arguments: String = "",
    host: String,
    workingDir: String = "",
    autoStart: Bool = false,
    proxyPort: UInt16? = nil,
    status: ServiceStatus
  ) {
    self.id = id
    self.name = name
    self.command = command
    self.arguments = arguments
    self.host = host
    self.workingDir = workingDir
    self.autoStart = autoStart
    if let proxyPort {
      self.localURL = "http://\(host).localhost:\(proxyPort)"
    } else {
      self.localURL = "http://\(host).localhost"
    }
    self.status = status
  }
}

public struct ServiceDefinition: Codable, Equatable {
  public let id: String
  public let name: String
  public let host: String
  public let run: String
  public let workingDir: String
  public let autoStart: Bool

  public init(
    id: String,
    name: String,
    host: String,
    run: String,
    workingDir: String,
    autoStart: Bool
  ) {
    self.id = id
    self.name = name
    self.host = host
    self.run = run
    self.workingDir = workingDir
    self.autoStart = autoStart
  }

  enum CodingKeys: String, CodingKey {
    case id
    case name
    case host
    case run
    case workingDir = "working_dir"
    case autoStart = "auto_start"
  }
}

public enum ServiceStatus: Equatable {
  case stopped
  case starting
  case running
  case failed

  public var isRunning: Bool { self == .running }
  public var isActive: Bool { self == .starting || self == .running || self == .failed }
}

public enum RackProxy {
  public static let fallbackPort = 8080
}
