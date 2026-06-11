import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = ServerStore()
    let launchAtLogin = LaunchAtLoginController()
    private let proxy = ProxyServer()
    private let ipc = IPCServer()
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let popover = NSPopover()

    func applicationDidFinishLaunching(_ notification: Notification) {
        CLIInstaller.installBundledCLI()
        configureMenuBarWindow()

        RackCore.shared.start { event in
            print("RackCore \(event)")
        }
        if let snapshot = RackCore.shared.command(#"{"type":"state.snapshot"}"#) {
            print("RackCore \(snapshot)")
        }
        store.reloadFunctions()

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
        RackCore.shared.stop()
        store.stopAllServers()
    }

    private func configureMenuBarWindow() {
        if let button = statusItem.button {
            button.image = NSImage(systemSymbolName: "server.rack", accessibilityDescription: "Rack")
            button.action = #selector(togglePopover(_:))
            button.target = self
        }

        popover.behavior = .transient
        popover.animates = true
        let controller = NSHostingController(
            rootView: MenuBarRootView()
                .environmentObject(store)
                .environmentObject(launchAtLogin)
        )
        controller.sizingOptions = [.preferredContentSize]
        popover.contentViewController = controller
    }

    @objc private func togglePopover(_ sender: Any?) {
        guard let button = statusItem.button else { return }
        if popover.isShown {
            popover.performClose(sender)
        } else {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
            popover.contentViewController?.view.window?.makeKey()
        }
    }
}

@MainActor
private struct MenuBarRootView: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        MenuBarContentView {
            openWindow(id: "settings")
            NSApplication.shared.activate(ignoringOtherApps: true)
        }
    }
}

@MainActor
@main
struct RackApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup("Rack. Settings", id: "settings") {
            SettingsView()
                .environmentObject(appDelegate.store)
                .environmentObject(appDelegate.launchAtLogin)
                .frame(minWidth: 860, minHeight: 540)
        }
        .windowResizability(.contentMinSize)
    }
}
