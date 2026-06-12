import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = ServerStore()
    let launchAtLogin = LaunchAtLoginController()
    private let proxy = ProxyServer()
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let popover = NSPopover()

    func applicationDidFinishLaunching(_ notification: Notification) {
        CLIInstaller.installBundledCLI()
        configureMenuBarWindow()

        RackCore.shared.start { [weak self] event in
            print("RackCore \(event)")
            self?.handleCoreEvent(event)
        }
        if let snapshot = RackCore.shared.command(#"{"type":"state.snapshot"}"#) {
            print("RackCore \(snapshot)")
        }
        store.reloadFunctions()
        store.syncIPCContext()

        Task {
            do {
                try await proxy.start()
                store.syncIPCContext()
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

    private func handleCoreEvent(_ event: String) {
        guard let data = event.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let eventType = object["type"] as? String,
              let payload = object["payload"] as? [String: Any]
        else { return }

        switch eventType {
        case "ipc.action":
            guard let type = payload["type"] as? String,
                  let id = payload["id"] as? String
            else { return }
            store.applyIPCHostAction(type: type, idString: id)

        case "server.output":
            guard let idString = payload["id"] as? String,
                  let id = UUID(uuidString: idString),
                  let output = payload["output"] as? String
            else { return }
            store.appendServerOutput(id: id, output: output)

        case "server.exited":
            guard let idString = payload["id"] as? String,
                  let id = UUID(uuidString: idString),
                  let status = payload["status"] as? Int
            else { return }
            let plan = decodeServerLaunchPlan(payload["plan"])
            store.handleServerExit(id: id, status: Int32(status), plan: plan)

        default:
            break
        }
    }

    private func decodeServerLaunchPlan(_ value: Any?) -> ServerLaunchPlan? {
        guard let value, JSONSerialization.isValidJSONObject(value),
              let data = try? JSONSerialization.data(withJSONObject: value)
        else { return nil }
        return try? JSONDecoder().decode(ServerLaunchPlan.self, from: data)
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
