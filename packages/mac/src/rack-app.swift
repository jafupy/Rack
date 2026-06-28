import RackUI
import SwiftUI

@main
struct RackApp: App {
  @NSApplicationDelegateAdaptor(RackAppDelegate.self) private var appDelegate

  var body: some Scene {
    MenuBarExtra("Rack", systemImage: "server.rack") {
      MenuBarContentView { section in
        appDelegate.openSettings(section: section)
      }
      .environmentObject(appDelegate.model)
    }
    .menuBarExtraStyle(.window)
  }
}
