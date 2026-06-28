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
    let rootView = RackSettingsView(initialSection: section)
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
