import RackUI
import SwiftUI

@main
struct RackApp: App {
  @StateObject private var store = ServerStore()

  var body: some Scene {
    MenuBarExtra("Rack", systemImage: "server.rack") {
      MenuBarContentView()
        .environmentObject(store)
    }
    .menuBarExtraStyle(.window)
  }
}
