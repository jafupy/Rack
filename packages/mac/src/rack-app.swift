import RackUI
import SwiftUI

@main
struct RackApp: App {
  @StateObject private var model = RackViewModel(runtime: RackServicesClient())

  var body: some Scene {
    MenuBarExtra("Rack", systemImage: "server.rack") {
      MenuBarContentView()
        .environmentObject(model)
        .task {
          RackIntentBridge.model = model
        }
    }
    .menuBarExtraStyle(.window)
  }
}
