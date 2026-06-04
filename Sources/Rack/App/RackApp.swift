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
    private var settingsWindow: NSWindow?

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
            rootView: MenuBarContentView(openSettings: { [weak self] in
                self?.showSettingsWindow()
            })
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

    private func showSettingsWindow() {
        if let settingsWindow {
            settingsWindow.makeKeyAndOrderFront(nil)
            NSApplication.shared.activate(ignoringOtherApps: true)
            return
        }

        let controller = NSHostingController(
            rootView: SettingsView()
                .environmentObject(store)
                .environmentObject(launchAtLogin)
                .frame(minWidth: 860, minHeight: 540)
        )
        let window = NSWindow(contentViewController: controller)
        window.title = "Settings"
        window.setContentSize(NSSize(width: 860, height: 540))
        window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
        window.isReleasedWhenClosed = false
        window.center()
        window.makeKeyAndOrderFront(nil)
        settingsWindow = window
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}

@MainActor
@main
struct RackApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup("Settings", id: "main") {
            SettingsView()
                .environmentObject(appDelegate.store)
                .environmentObject(appDelegate.launchAtLogin)
                .frame(minWidth: 860, minHeight: 540)
        }
    }
}
