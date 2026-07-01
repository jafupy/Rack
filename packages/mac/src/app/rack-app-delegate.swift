import AppKit
import Foundation
import RackUI
import SwiftUI

@MainActor
final class RackAppDelegate: NSObject, NSApplicationDelegate {
  let model: RackViewModel

  private let runtime: RackServicesClient

  override init() {
    runtime = RackServicesClient()
    model = RackViewModel(runtime: runtime)
    super.init()
    RackIntentBridge.model = model
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    RackIntentBridge.model = model
  }

  func applicationWillTerminate(_ notification: Notification) {
    model.stopAll()
    runtime.shutdown()
  }

  func application(_ application: NSApplication, open urls: [URL]) {
    urls.forEach(handleURL)
  }

  func openSettings(section: SettingsSection = .general) {
    SettingsWindowController.shared.open(model: model, section: section)
  }

  private func handleURL(_ url: URL) {
    guard url.scheme?.lowercased() == "rack" else { return }

    switch RackURLAction(url: url) {
    case .settings:
      openSettings()
    case .startService(let id):
      model.start(id: id)
    case .stopService(let id):
      model.stop(id: id)
    case .restartService(let id):
      model.restart(id: id)
    case .stopAllServices:
      model.stopAll()
    case .reloadHooks:
      model.reloadHooks()
    case .none:
      print("unsupported Rack URL: \(url.absoluteString)")
    }
  }
}

private enum RackURLAction {
  case settings
  case startService(String)
  case stopService(String)
  case restartService(String)
  case stopAllServices
  case reloadHooks

  init?(url: URL) {
    let host = url.host?.lowercased()
    let path = url.pathComponents
      .filter { $0 != "/" }
      .map { $0.lowercased() }
    let rawPath = url.pathComponents.filter { $0 != "/" }
    let id = url.queryValue(named: "id") ?? url.queryValue(named: "service")

    if host == "settings" || path.first == "settings" {
      self = .settings
      return
    }

    if host == "hooks" || host == "functions", path.first == "reload" {
      self = .reloadHooks
      return
    }

    if host == "reload-hooks" || host == "hooks-reload" || host == "functions-reload" {
      self = .reloadHooks
      return
    }

    if ["services", "service", "servers", "server"].contains(host), path.first == "stop-all" {
      self = .stopAllServices
      return
    }

    if host == "stop-all" || host == "stop-all-services" {
      self = .stopAllServices
      return
    }

    if ["service", "services", "server", "servers"].contains(host) {
      guard let action = path.first else { return nil }
      let pathID = rawPath.dropFirst().first
      self.init(serviceAction: action, id: id ?? pathID)
      return
    }

    if let host, ["start", "stop", "restart"].contains(host) {
      self.init(serviceAction: host, id: id ?? rawPath.first)
      return
    }

    return nil
  }

  private init?(serviceAction action: String, id: String?) {
    guard let id, !id.isEmpty else { return nil }

    switch action {
    case "start", "start-service":
      self = .startService(id)
    case "stop", "stop-service":
      self = .stopService(id)
    case "restart", "restart-service":
      self = .restartService(id)
    default:
      return nil
    }
  }
}

extension URL {
  fileprivate func queryValue(named name: String) -> String? {
    URLComponents(url: self, resolvingAgainstBaseURL: false)?
      .queryItems?
      .first { $0.name == name }?
      .value
  }
}
