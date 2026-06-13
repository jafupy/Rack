import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = ServerStore()
    let launchAtLogin = LaunchAtLoginController()
    private let proxy = ProxyServer()
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    private let popover = NSPopover()
    private var settingsWindowController: NSWindowController?

    func applicationWillFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.accessory)
    }

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

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls {
            handleRackURL(url)
        }
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
            rootView: MenuBarContentView { [weak self] in
                self?.popover.performClose(nil)
                self?.showSettingsWindow()
            }
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

        case "server.ready":
            guard let idString = payload["id"] as? String,
                  let id = UUID(uuidString: idString),
                  let pid = payload["pid"] as? Int
            else { return }
            store.markServerReady(id: id, pid: Int32(pid))

        case "server.failed":
            guard let idString = payload["id"] as? String,
                  let id = UUID(uuidString: idString)
            else { return }
            store.markServerFailed(
                id: id,
                message: (payload["message"] as? String) ?? "Server failed"
            )

        default:
            break
        }
    }

    private func handleRackURL(_ url: URL) {
        guard url.scheme?.lowercased() == "rack" else { return }

        let host = url.host()?.lowercased()
        let pathComponents = url.pathComponents.filter { $0 != "/" }.map { $0.lowercased() }
        let action = ([host] + pathComponents).compactMap { $0 }.joined(separator: ".")

        switch action {
        case "settings", "settings.open", "open-settings":
            showSettingsWindow()
        case "server.start", "servers.start", "start":
            guard let id = serverID(from: url) else { return }
            store.startServer(id: id)
        case "server.stop", "servers.stop", "stop":
            guard let id = serverID(from: url) else { return }
            store.stopServer(id: id)
        case "server.restart", "servers.restart", "restart":
            guard let id = serverID(from: url) else { return }
            store.restartServer(id: id)
        case "server.stop-all", "servers.stop-all", "stop-all":
            store.stopAllServers()
        case "functions.reload", "reload-functions":
            store.reloadFunctions()
        default:
            break
        }
    }

    private func serverID(from url: URL) -> UUID? {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
            return nil
        }

        let queryItems = components.queryItems ?? []
        if let idValue = queryItems.first(where: { $0.name == "id" })?.value,
           let id = UUID(uuidString: idValue) {
            return id
        }

        if let name = queryItems.first(where: { $0.name == "name" })?.value?.lowercased() {
            return store.servers.first { server in
                server.name.lowercased() == name || server.routeSubdomain.lowercased() == name
            }?.id
        }

        if let lastPathComponent = url.pathComponents.last,
           let id = UUID(uuidString: lastPathComponent) {
            return id
        }

        return nil
    }

    private func decodeServerLaunchPlan(_ value: Any?) -> ServerLaunchPlan? {
        guard let value, JSONSerialization.isValidJSONObject(value),
              let data = try? JSONSerialization.data(withJSONObject: value)
        else { return nil }
        return try? JSONDecoder().decode(ServerLaunchPlan.self, from: data)
    }

    private func showSettingsWindow() {
        if settingsWindowController == nil {
            let controller = NSHostingController(
                rootView: SettingsView()
                    .environmentObject(store)
                    .environmentObject(launchAtLogin)
                    .frame(minWidth: 860, minHeight: 540)
            )
            let window = NSWindow(contentViewController: controller)
            window.title = "Rack. Settings"
            window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
            window.minSize = NSSize(width: 860, height: 540)
            window.isReleasedWhenClosed = false
            window.center()
            settingsWindowController = NSWindowController(window: window)
        }

        settingsWindowController?.showWindow(nil)
        settingsWindowController?.window?.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }
}

@MainActor
@main
struct RackApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        Settings {
            EmptyView()
        }
    }
}
