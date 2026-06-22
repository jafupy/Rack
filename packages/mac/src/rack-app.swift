import RackUI
import SwiftUI

@main
struct RackApp: App {
  @StateObject private var model = RackViewModel(runtime: RackServicesClient())

  var body: some Scene {
    MenuBarExtra("Rack", systemImage: "server.rack") {
      MenuBarContentView()
        .environmentObject(model)
    }
    .menuBarExtraStyle(.window)
  }
}
