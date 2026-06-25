import Foundation
import ServiceManagement
import SwiftUI

@MainActor
final class LaunchAtLoginController: ObservableObject {
  @Published private(set) var isEnabled = false
  @Published private(set) var errorMessage: String?

  init() {
    refresh()
  }

  func refresh() {
    errorMessage = nil
    isEnabled = Self.isEnabledStatus(SMAppService.mainApp.status)
  }

  func setEnabled(_ enabled: Bool) {
    errorMessage = nil

    do {
      if enabled {
        try SMAppService.mainApp.register()
      } else {
        try SMAppService.mainApp.unregister()
      }
      refresh()
      if enabled, !isEnabled {
        errorMessage = Self.message(for: SMAppService.mainApp.status)
      }
    } catch {
      refresh()
      errorMessage = Self.message(for: SMAppService.mainApp.status) ?? error.localizedDescription
    }
  }

  private static func isEnabledStatus(_ status: SMAppService.Status) -> Bool {
    switch status {
    case .enabled, .requiresApproval:
      true
    case .notFound, .notRegistered:
      false
    @unknown default:
      false
    }
  }

  private static func message(for status: SMAppService.Status) -> String? {
    switch status {
    case .requiresApproval:
      "macOS requires approval for launch at login in System Settings."
    case .notFound:
      "Launch at login is only available from the built app bundle."
    case .enabled, .notRegistered:
      nil
    @unknown default:
      "Rack could not update its login item state."
    }
  }
}
