import AppIntents

struct ReloadRackHooksIntent: AppIntent {
  static let title: LocalizedStringResource = "Reload Rack Hooks"
  static let description = IntentDescription("Refresh Rack hook summaries.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    guard let model = RackIntentBridge.model else { throw RackIntentError.appUnavailable }
    model.reloadHooks()
    return .result(dialog: "Reloaded Rack hooks")
  }
}

struct EnableRackLaunchAtLoginIntent: AppIntent {
  static let title: LocalizedStringResource = "Enable Rack Launch at Login"
  static let description = IntentDescription("Register Rack as a login item.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    RackIntentBridge.launchAtLogin.setEnabled(true)
    return .result(dialog: "Enabled Rack launch at login")
  }
}

struct DisableRackLaunchAtLoginIntent: AppIntent {
  static let title: LocalizedStringResource = "Disable Rack Launch at Login"
  static let description = IntentDescription("Remove Rack from login items.")
  static let openAppWhenRun = true

  @MainActor
  func perform() async throws -> some IntentResult & ProvidesDialog {
    RackIntentBridge.launchAtLogin.setEnabled(false)
    return .result(dialog: "Disabled Rack launch at login")
  }
}
