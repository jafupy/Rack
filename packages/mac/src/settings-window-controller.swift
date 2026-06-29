import AppKit
import RackUI
import SwiftUI

@MainActor
final class SettingsWindowController {
  static let shared = SettingsWindowController()

  private var window: NSWindow?
  private var windowDelegate: SettingsWindowDelegate?

  private init() {}

  func open(model: RackViewModel, section: SettingsSection = .general) {
    let rootView = RackSettingsView(
      initialSection: section,
      generalState: generalState(model: model),
      generalActions: generalActions(model: model)
    )
    .environmentObject(model)

    if let window {
      window.contentViewController = NSHostingController(rootView: rootView)
      window.makeKeyAndOrderFront(nil)
      NSApp.activate(ignoringOtherApps: true)
      return
    }

    let hostingController = NSHostingController(rootView: rootView)
    let window = NSWindow(contentViewController: hostingController)
    window.title = "Rack Settings"
    window.setContentSize(NSSize(width: 820, height: 560))
    window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
    window.isReleasedWhenClosed = false
    window.center()

    let delegate = SettingsWindowDelegate { [weak self] in
      self?.window = nil
      self?.windowDelegate = nil
    }
    window.delegate = delegate
    windowDelegate = delegate
    self.window = window

    window.makeKeyAndOrderFront(nil)
    NSApp.activate(ignoringOtherApps: true)
  }

  private func generalState(model: RackViewModel) -> GeneralSettingsState {
    let launch = RackIntentBridge.launchAtLogin
    launch.refresh()
    return GeneralSettingsState(
      launchAtLoginEnabled: launch.isEnabled,
      launchAtLoginMessage: launch.errorMessage,
      terminalName: model.terminalName(),
      configPath: model.configPath()
    )
  }

  private func generalActions(model: RackViewModel) -> GeneralSettingsActions {
    GeneralSettingsActions(
      setLaunchAtLogin: { enabled in
        let launch = RackIntentBridge.launchAtLogin
        launch.setEnabled(enabled)
        return launch.errorMessage
      },
      setTerminalName: { terminal in
        try model.setTerminalName(terminal)
      },
      revealConfig: {
        guard let path = model.configPath() else { return }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
      },
      openConfig: {
        guard let path = model.configPath() else { return }
        NSWorkspace.shared.open(URL(fileURLWithPath: path))
      },
      installCLI: {
        CLIInstaller().install()
      }
    )
  }
}

private final class SettingsWindowDelegate: NSObject, NSWindowDelegate {
  private let didClose: () -> Void

  init(didClose: @escaping () -> Void) {
    self.didClose = didClose
  }

  func windowWillClose(_ notification: Notification) {
    didClose()
  }
}
