import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = ServerStore()
    private let proxy = ProxyServer()
    private let ipc = IPCServer()

    func applicationDidFinishLaunching(_ notification: Notification) {
        ipc.store = store
        ipc.start()
        Task {
            do {
                try await proxy.start()
            } catch {
                print("RackProxy failed to start: \(error)")
                store.stopAllServers()
            }
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        store.stopAllServers()
    }
}

@MainActor
@main
struct RackApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var launchAtLogin = LaunchAtLoginController()

    var body: some Scene {
        MenuBarExtra("Rack.", systemImage: "server.rack") {
            MenuBarContentView()
                .environmentObject(appDelegate.store)
                .environmentObject(launchAtLogin)
        }
        .menuBarExtraStyle(.window)

        WindowGroup("Settings", id: "main") {
            SettingsView()
                .environmentObject(appDelegate.store)
                .environmentObject(launchAtLogin)
                .frame(minWidth: 860, minHeight: 540)
        }
    }
}
