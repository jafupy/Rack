import AppIntents
import RackUI

struct StartRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Start Rack Service"
  static let description = IntentDescription("Start one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.start(id: service.id)
    return .result(dialog: "Starting \(service.name)")
  }
}

struct StopRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Stop Rack Service"
  static let description = IntentDescription("Stop one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.stop(id: service.id)
    return .result(dialog: "Stopped \(service.name)")
  }
}

struct RestartRackServiceIntent: AppIntent {
  static let title: LocalizedStringResource = "Restart Rack Service"
  static let description = IntentDescription("Restart one configured Rack service.")
  static let openAppWhenRun = true

  @Parameter(title: "Service")
  var service: RackServiceEntity

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    guard try RackIntentBridge.service(id: service.id) != nil else {
      throw RackIntentError.serviceNotFound
    }
    model.restart(id: service.id)
    return .result(dialog: "Restarting \(service.name)")
  }
}

struct StopAllRackServicesIntent: AppIntent {
  static let title: LocalizedStringResource = "Stop All Rack Services"
  static let description = IntentDescription("Stop every active Rack service.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    model.stopAll()
    return .result(dialog: "Stopped all Rack services")
  }
}
