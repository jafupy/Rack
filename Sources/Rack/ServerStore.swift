import AppKit
import Darwin
import Foundation
import SwiftUI

@MainActor
final class ServerStore: ObservableObject {
    private enum AppPaths {
        static let appName = "Rack."
        static let temporaryDirectoryName = "Rack"
        static let commandFilePrefix = "rack"
        static let storageDirectoryName = "rack"
        static let legacyStorageDirectoryName = "server-bar"
        static let legacyAppSupportDirectoryName = "ServerBar"
        static let legacyDefaultsBundleID = "dev.jafu.ServerBar"
    }

    @Published var servers: [ServerConfiguration] = []
    @Published var selectedServerID: ServerConfiguration.ID?
    @Published private(set) var statuses: [ServerConfiguration.ID: ServerStatus] = [:]
    @Published private(set) var logs: [ServerConfiguration.ID: String] = [:]

    private var processes: [ServerConfiguration.ID: ServerProcess] = [:]
    private var logFilePaths: [ServerConfiguration.ID: URL] = [:]
    private var logFileHandles: [ServerConfiguration.ID: FileHandle] = [:]
    private var terminationSignalSources: [DispatchSourceSignal] = []
    private var isHandlingTerminationSignal = false
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    init() {
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        load()

        if selectedServerID == nil {
            selectedServerID = servers.first?.id
        }

        Task {
            autoStartServers()
        }

        installTerminationSignalHandlers()
    }

    var selectedServer: Binding<ServerConfiguration>? {
        guard let selectedServerID else { return nil }
        return binding(for: selectedServerID)
    }

    var configurationURL: URL {
        let dir = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".config/\(AppPaths.storageDirectoryName)")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appending(path: "config.json")
    }

    func addServer() {
        let server = ServerConfiguration()
        addServer(server)
    }

    func addServer(_ server: ServerConfiguration) {
        servers.append(server)
        selectedServerID = server.id
        statuses[server.id] = .stopped
        save()
    }

    func deleteServers(at offsets: IndexSet) {
        for index in offsets {
            let id = servers[index].id
            stopServer(id: id)
            statuses[id] = nil
            logs[id] = nil
        }

        servers.remove(atOffsets: offsets)
        if !servers.contains(where: { $0.id == selectedServerID }) {
            selectedServerID = servers.first?.id
        }
        save()
    }

    func duplicateSelectedServer() {
        guard let selectedServer else { return }
        var copy = selectedServer.wrappedValue
        copy.id = UUID()
        copy.name += " Copy"
        servers.append(copy)
        selectedServerID = copy.id
        statuses[copy.id] = .stopped
        save()
    }

    func deleteSelectedServer() {
        guard let selectedServerID, let index = servers.firstIndex(where: { $0.id == selectedServerID }) else {
            return
        }

        deleteServers(at: IndexSet(integer: index))
    }

    func binding(for id: ServerConfiguration.ID) -> Binding<ServerConfiguration>? {
        guard servers.contains(where: { $0.id == id }) else {
            return nil
        }

        return Binding(
            get: {
                self.servers.first(where: { $0.id == id }) ?? ServerConfiguration(id: id)
            },
            set: { value in
                guard let index = self.servers.firstIndex(where: { $0.id == id }) else {
                    return
                }
                self.servers[index] = value
                self.save()
            }
        )
    }

    func status(for id: ServerConfiguration.ID) -> ServerStatus {
        statuses[id] ?? .stopped
    }

    func log(for id: ServerConfiguration.ID) -> String {
        logs[id] ?? ""
    }

    func startServer(id: ServerConfiguration.ID) {
        guard let config = servers.first(where: { $0.id == id }) else { return }
        guard !config.command.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            statuses[id] = .failed(message: "Missing command")
            return
        }
        guard processes[id] == nil else { return }

        statuses[id] = .starting
        logs[id] = ""

        // Assign an internal loopback port for rack-bridge
        let bridgePort = allocatePort()
        let socketPath = socketPath(for: config)

        // Register the route so the proxy can start routing immediately
        let route = Route(
            name: routeName(for: config),
            socketPath: socketPath,
            workingDirectory: config.workingDirectory,
            addedAt: .now
        )
        RouteRegistry.shared.register(route)

        // Create a fresh temp log file for this run
        let tmpDir = URL(fileURLWithPath: NSTemporaryDirectory()).appending(path: AppPaths.temporaryDirectoryName)
        try? FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
        let logURL = tmpDir.appending(path: "\(id.uuidString).log")
        FileManager.default.createFile(atPath: logURL.path, contents: nil)
        logFilePaths[id] = logURL
        logFileHandles[id] = try? FileHandle(forWritingTo: logURL)

        // Build a bridged config that wraps the real command with rack-bridge
        let bridgedConfig = makeBridgedConfig(config, socketPath: socketPath, bridgePort: bridgePort)

        let process = ServerProcess(
            configuration: bridgedConfig,
            outputHandler: { [weak self] output in
                guard let self else { return }
                self.logs[id, default: ""] += output
                if let handle = self.logFileHandles[id], let data = output.data(using: .utf8) {
                    try? handle.write(contentsOf: data)
                }
                let components = self.logs[id, default: ""].split(separator: "\n", omittingEmptySubsequences: false)
                if components.count > 400 {
                    self.logs[id] = components.suffix(400).joined(separator: "\n")
                }
            },
            exitHandler: { [weak self] status in
                guard let self else { return }
                self.processes[id] = nil
                self.statuses[id] = status == 0 ? .stopped : .failed(message: "Exit \(status)")
                // Clean up the route when the server exits
                RouteRegistry.shared.unregister(name: self.routeName(for: config))
            }
        )

        do {
            try process.start()
            processes[id] = process
            statuses[id] = .running(pid: process.process.processIdentifier)
        } catch {
            statuses[id] = .failed(message: error.localizedDescription)
            RouteRegistry.shared.unregister(name: routeName(for: config))
        }
    }

    func stopServer(id: ServerConfiguration.ID) {
        if let config = servers.first(where: { $0.id == id }) {
            RouteRegistry.shared.unregister(name: routeName(for: config))
        }
        processes[id]?.stop()
        processes[id] = nil
        statuses[id] = .stopped
        try? logFileHandles[id]?.close()
        logFileHandles[id] = nil
    }

    func logFilePath(for id: ServerConfiguration.ID) -> URL? {
        logFilePaths[id]
    }

    func openInTerminal(id: ServerConfiguration.ID) {
        guard let logURL = logFilePaths[id] else { return }
        let logPath = logURL.path
        let appName = UserDefaults.standard.string(forKey: "terminalApp") ?? "Ghostty"

        // Single-quoted shell-safe path
        let safePath = logPath.replacingOccurrences(of: "'", with: "'\\''")
        let tailCmd = "tail -n 200 -f '\(safePath)'"

        func run(_ executable: String, args: [String]) {
            let p = Process()
            p.executableURL = URL(fileURLWithPath: executable)
            p.arguments = args
            try? p.run()
        }

        func runAppleScript(_ source: String) {
            run("/usr/bin/osascript", args: ["-e", source])
        }

        func escapeAppleScriptString(_ value: String) -> String {
            value
                .replacingOccurrences(of: "\\", with: "\\\\")
                .replacingOccurrences(of: "\"", with: "\\\"")
        }

        // Write a temp .command file (executable shell script) for apps that support it
        func commandFileURL() -> URL? {
            let url = URL(fileURLWithPath: NSTemporaryDirectory())
                .appending(path: "\(AppPaths.commandFilePrefix)-\(id.uuidString).command")
            let content = "#!/bin/sh\n\(tailCmd)\n"
            guard (try? content.write(to: url, atomically: true, encoding: .utf8)) != nil else { return nil }
            try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
            return url
        }

        switch appName.lowercased() {
        case "ghostty":
            let escapedTailCmd = escapeAppleScriptString(tailCmd)
            runAppleScript("""
                tell application "Ghostty"
                    activate
                    set cfg to new surface configuration
                    if (count of windows) = 0 then
                        set win to new window with configuration cfg
                        set term to focused terminal of selected tab of win
                    else
                        set win to front window
                        set newTab to new tab in win with configuration cfg
                        set term to focused terminal of newTab
                    end if
                    input text "\(escapedTailCmd)" to term
                    send key "enter" to term
                end tell
                """)

        case "terminal":
            runAppleScript("""
                tell application "Terminal"
                    do script "\(escapeAppleScriptString(tailCmd))"
                    activate
                end tell
                """)

        case "iterm", "iterm2":
            runAppleScript("""
                tell application "iTerm2"
                    activate
                    set w to (create window with default profile)
                    tell current session of w
                        write text "\(escapeAppleScriptString(tailCmd))"
                    end tell
                end tell
                """)

        case "warp":
            if let url = commandFileURL() {
                run("/usr/bin/open", args: ["-a", "Warp", url.path])
            }

        default:
            // Generic: open a .command file with the named app
            if let url = commandFileURL() {
                run("/usr/bin/open", args: ["-a", appName, url.path])
            }
        }
    }

    func restartServer(id: ServerConfiguration.ID) {
        stopServer(id: id)
        startServer(id: id)
    }

    func stopAllServers() {
        for server in servers {
            stopServer(id: server.id)
        }
    }

    func revealConfigurationFile() {
        NSWorkspace.shared.activateFileViewerSelecting([configurationURL])
    }

    private func autoStartServers() {
        for server in servers where server.autoStart {
            startServer(id: server.id)
        }
    }

    private func load() {
        migrateIfNeeded()
        guard FileManager.default.fileExists(atPath: configurationURL.path()) else {
            servers = []
            return
        }

        do {
            let data = try Data(contentsOf: configurationURL)
            let configuration = try decoder.decode(PersistedConfiguration.self, from: data)
            servers = configuration.servers
            for server in servers {
                statuses[server.id] = .stopped
            }
        } catch {
            servers = []
        }
    }

    private func migrateIfNeeded() {
        migrateConfigurationIfNeeded()
        migrateDefaultsIfNeeded()
    }

    private func migrateConfigurationIfNeeded() {
        let fileManager = FileManager.default
        let new = configurationURL
        guard !fileManager.fileExists(atPath: new.path) else { return }

        let legacyCandidates = [
            FileManager.default.homeDirectoryForCurrentUser
                .appending(path: ".config/\(AppPaths.legacyStorageDirectoryName)/config.json"),
            fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
                .appending(path: "\(AppPaths.legacyAppSupportDirectoryName)/servers.json"),
        ]

        guard let old = legacyCandidates.first(where: { fileManager.fileExists(atPath: $0.path) }) else { return }
        try? fileManager.createDirectory(at: new.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? fileManager.copyItem(at: old, to: new)
    }

    private func migrateDefaultsIfNeeded() {
        let defaults = UserDefaults.standard
        if defaults.object(forKey: "terminalApp") == nil,
           let legacyDefaults = defaults.persistentDomain(forName: AppPaths.legacyDefaultsBundleID),
           let terminalApp = legacyDefaults["terminalApp"] as? String {
            defaults.set(terminalApp, forKey: "terminalApp")
        }
    }

    private func save() {
        do {
            let data = try encoder.encode(PersistedConfiguration(servers: servers))
            try data.write(to: configurationURL, options: .atomic)
        } catch {
            NSSound.beep()
        }
    }

    // MARK: - Bridge helpers

    private func routeName(for config: ServerConfiguration) -> String {
        config.name.lowercased().replacingOccurrences(of: " ", with: "-")
    }

    private func socketPath(for config: ServerConfiguration) -> String {
        let sockDir = "/tmp/rack"
        try? FileManager.default.createDirectory(atPath: sockDir, withIntermediateDirectories: true)
        return "\(sockDir)/\(routeName(for: config)).sock"
    }

    private func allocatePort() -> Int {
        let used = Set(RouteRegistry.shared.allRoutes().compactMap { _ in Int?.none }) // routes don't store port
        for port in (4000...4999).shuffled() {
            if !isPortInUse(port) { return port }
        }
        return 4000
    }

    private func isPortInUse(_ port: Int) -> Bool {
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = in_port_t(port).bigEndian
        addr.sin_addr.s_addr = INADDR_LOOPBACK.bigEndian
        let sock = socket(AF_INET, SOCK_STREAM, 0)
        defer { close(sock) }
        return withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                connect(sock, $0, socklen_t(MemoryLayout<sockaddr_in>.size)) == 0
            }
        }
    }

    /// Wraps the user's command in rack-bridge so it connects via unix socket.
    private func makeBridgedConfig(_ config: ServerConfiguration, socketPath: String, bridgePort: Int) -> ServerConfiguration {
        let bridgePath = Bundle.main.path(forResource: "rack-bridge", ofType: nil)
            ?? "/usr/local/bin/rack-bridge"

        var bridged = config
        // rack-bridge --socket <path> --port <n> -- <original command>
        bridged.command = bridgePath
        bridged.arguments = "--socket \(socketPath) --port \(bridgePort) -- \(config.command) \(config.arguments)"
        return bridged
    }

    private func installTerminationSignalHandlers() {
        let signals = [SIGTERM, SIGINT, SIGHUP]

        for signalNumber in signals {
            signal(signalNumber, SIG_IGN)

            let source = DispatchSource.makeSignalSource(signal: signalNumber, queue: .main)
            source.setEventHandler { [weak self] in
                guard let self, !self.isHandlingTerminationSignal else { return }
                self.isHandlingTerminationSignal = true
                self.stopAllServers()
                NSApp.terminate(nil)
            }
            source.resume()
            terminationSignalSources.append(source)
        }
    }
}
