import AppKit
import SwiftUI

@MainActor
@main
struct RackApp: App {
    @StateObject private var store = ServerStore()

    var body: some Scene {
        MenuBarExtra("Rack.", systemImage: "server.rack") {
            MenuBarContentView()
                .environmentObject(store)
        }
        .menuBarExtraStyle(.window)

        WindowGroup("Configure Servers", id: "main") {
            SettingsView()
                .environmentObject(store)
                .frame(minWidth: 860, minHeight: 540)
        }
    }
}
