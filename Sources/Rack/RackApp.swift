import AppKit
import SwiftUI

final class AppDelegate: NSObject, NSApplicationDelegate {
    weak var store: ServerStore?

    func applicationWillTerminate(_ notification: Notification) {
        store?.stopAllServers()
    }
}

@MainActor
@main
struct RackApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var store = ServerStore()
    @StateObject private var launchAtLogin = LaunchAtLoginController()

    private let proxy = ProxyServer()
    private let ipc = IPCServer()

    var body: some Scene {
        MenuBarExtra("Rack.", systemImage: "server.rack") {
            MenuBarContentView()
                .environmentObject(store)
                .environmentObject(launchAtLogin)
                .task {
                    appDelegate.store = store
                    ipc.store = store
                    ipc.start()
                    do {
                        try await proxy.start()
                    } catch {
                        // Non-fatal: app works without proxy, just no .localhost URLs
                        print("RackProxy failed to start: \(error)")
                    }
                }
        }
        .menuBarExtraStyle(.window)

        WindowGroup("Settings", id: "main") {
            SettingsView()
                .environmentObject(store)
                .environmentObject(launchAtLogin)
                .frame(minWidth: 860, minHeight: 540)
                .task {
                    appDelegate.store = store
                }
        }
    }
}
