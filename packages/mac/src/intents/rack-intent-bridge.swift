import Foundation
import RackUI

@MainActor
enum RackIntentBridge {
  static var model: RackViewModel?
  static let launchAtLogin = LaunchAtLoginController()

  static func services() throws -> [ServiceConfiguration] {
    guard let model else { throw RackIntentError.appUnavailable }
    return model.services
  }

  static func service(id: String) throws -> ServiceConfiguration? {
    try services().first { $0.id == id }
  }
}

enum RackIntentError: LocalizedError {
  case appUnavailable
  case serviceNotFound

  var errorDescription: String? {
    switch self {
    case .appUnavailable: "Rack is not ready yet."
    case .serviceNotFound: "That Rack service no longer exists."
    }
  }
}
