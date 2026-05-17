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
    private var readyTasks: [ServerConfiguration.ID: Task<Void, Never>] = [:]
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

        let subdomain = config.routeSubdomain

        let port = config.port ?? allocatePort()
        let socketPath = Self.socketPath(for: subdomain)

        // Remove any stale socket from a previous run.
        try? FileManager.default.removeItem(atPath: socketPath)
        try? FileManager.default.createDirectory(
            atPath: "/tmp/rack", withIntermediateDirectories: true, attributes: nil)

        // Register route immediately with empty socketPath / zero tcpPort.
        // awaitServerReady fills these in once the server is listening.
        RouteRegistry.shared.register(Route(
            name: subdomain,
            socketPath: "",
            tcpPort: 0,
            workingDirectory: config.workingDirectory,
            addedAt: .now
        ))

        // Create a fresh temp log file for this run.
        let tmpDir = URL(fileURLWithPath: NSTemporaryDirectory()).appending(path: AppPaths.temporaryDirectoryName)
        try? FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
        let logURL = tmpDir.appending(path: "\(id.uuidString).log")
        FileManager.default.createFile(atPath: logURL.path, contents: nil)
        logFilePaths[id] = logURL
        logFileHandles[id] = try? FileHandle(forWritingTo: logURL)

        let useBridge = ServerProcess.findRackBridge() != nil

        let process = ServerProcess(
            configuration: config,
            socketPath: socketPath,
            port: port,
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
                RouteRegistry.shared.unregister(name: subdomain)
                try? FileManager.default.removeItem(atPath: socketPath)
            }
        )

        // For TCP fallback (no rack-bridge): snapshot ports before launch.
        let portSnapshot = (!useBridge && config.port == nil) ? ServerStore.loopbackListeningPorts() : []

        do {
            try process.start()
            processes[id] = process
            let pid = process.process.processIdentifier
            readyTasks[id] = Task { [weak self] in
                if useBridge {
                    await self?.awaitServerReadyViaSocket(
                        id: id, pid: pid, subdomain: subdomain, socketPath: socketPath)
                } else {
                    await self?.awaitServerReadyViaTCP(
                        id: id, pid: pid, subdomain: subdomain,
                        explicitPort: config.port ?? port, portSnapshot: portSnapshot)
                }
            }
        } catch {
            statuses[id] = .failed(message: error.localizedDescription)
            RouteRegistry.shared.unregister(name: subdomain)
        }
    }

    func stopServer(id: ServerConfiguration.ID) {
        readyTasks[id]?.cancel()
        readyTasks[id] = nil
        if let config = servers.first(where: { $0.id == id }) {
            let subdomain = config.routeSubdomain
            RouteRegistry.shared.unregister(name: subdomain)
            try? FileManager.default.removeItem(atPath: Self.socketPath(for: subdomain))
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

        var legacyCandidates = [
            FileManager.default.homeDirectoryForCurrentUser
                .appending(path: ".config/\(AppPaths.legacyStorageDirectoryName)/config.json"),
        ]
        if let appSupportURL = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first {
            legacyCandidates.append(
                appSupportURL.appending(path: "\(AppPaths.legacyAppSupportDirectoryName)/servers.json")
            )
        }

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

    // MARK: - Port / socket helpers

    private nonisolated static func socketPath(for subdomain: String) -> String {
        "/tmp/rack/\(subdomain).sock"
    }

    private func allocatePort() -> Int {
        for port in (4000...4999).shuffled() {
            if !ServerStore.probePort(port) { return port }
        }
        return 4000
    }

    /// Rack-bridge mode: poll for the unix socket to appear, then mark running.
    private func awaitServerReadyViaSocket(id: ServerConfiguration.ID, pid: Int32,
                                           subdomain: String, socketPath: String) async {
        for _ in 0..<120 {
            do { try await Task.sleep(for: .milliseconds(500)) } catch { return }
            guard statuses[id] == .starting else { return }
            let ready = await Task.detached(priority: .utility) { ServerStore.probeUnixSocket(socketPath) }.value
            if ready {
                RouteRegistry.shared.updateSocketPath(name: subdomain, socketPath: socketPath)
                statuses[id] = .running(pid: pid)
                return
            }
        }
        if statuses[id] == .starting {
            statuses[id] = .failed(message: "Did not start within 60s")
        }
    }

    /// TCP fallback (no rack-bridge): probe the port directly or discover a new loopback port.
    private func awaitServerReadyViaTCP(id: ServerConfiguration.ID, pid: Int32, subdomain: String,
                                        explicitPort: Int, portSnapshot: Set<Int>) async {
        // If port was explicitly configured, probe it directly.
        if portSnapshot.isEmpty {
            for _ in 0..<120 {
                do { try await Task.sleep(for: .milliseconds(500)) } catch { return }
                guard statuses[id] == .starting else { return }
                let up = await Task.detached(priority: .utility) { ServerStore.probePort(explicitPort) }.value
                if up {
                    RouteRegistry.shared.updatePort(name: subdomain, tcpPort: explicitPort)
                    statuses[id] = .running(pid: pid)
                    return
                }
            }
        } else {
            for _ in 0..<120 {
                do { try await Task.sleep(for: .milliseconds(500)) } catch { return }
                guard statuses[id] == .starting else { return }
                let current = await Task.detached(priority: .utility) {
                    ServerStore.loopbackListeningPorts()
                }.value
                let newPorts = current.subtracting(portSnapshot).filter { $0 > 1024 }
                if let port = newPorts.sorted().first {
                    RouteRegistry.shared.updatePort(name: subdomain, tcpPort: port)
                    statuses[id] = .running(pid: pid)
                    return
                }
            }
        }
        if statuses[id] == .starting {
            statuses[id] = .failed(message: "Did not start within 60s")
        }
    }

    private nonisolated static func probeUnixSocket(_ path: String) -> Bool {
        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else { return false }
        defer { close(fd) }
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        withUnsafeMutableBytes(of: &addr.sun_path) { dest in
            path.withCString { src in
                guard let base = dest.baseAddress else { return }
                strlcpy(base.assumingMemoryBound(to: CChar.self), src, dest.count)
            }
        }
        return withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size)) == 0
            }
        }
    }

    private nonisolated static func probePort(_ port: Int) -> Bool {
        if probeIPv4Port(port) { return true }
        return probeIPv6Port(port)
    }

    private nonisolated static func probeIPv4Port(_ port: Int) -> Bool {
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = in_port_t(port).bigEndian
        addr.sin_addr.s_addr = in_addr_t(0x7f000001).bigEndian
        let sock = socket(AF_INET, SOCK_STREAM, 0)
        guard sock >= 0 else { return false }
        defer { close(sock) }
        return withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                connect(sock, $0, socklen_t(MemoryLayout<sockaddr_in>.size)) == 0
            }
        }
    }

    private nonisolated static func probeIPv6Port(_ port: Int) -> Bool {
        var addr = sockaddr_in6()
        addr.sin6_family = sa_family_t(AF_INET6)
        addr.sin6_port = in_port_t(port).bigEndian
        addr.sin6_addr = in6addr_loopback
        let sock = socket(AF_INET6, SOCK_STREAM, 0)
        guard sock >= 0 else { return false }
        defer { close(sock) }
        return withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                connect(sock, $0, socklen_t(MemoryLayout<sockaddr_in6>.size)) == 0
            }
        }
    }

    private nonisolated static func loopbackListeningPorts() -> Set<Int> {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/sbin/lsof")
        p.arguments = ["-iTCP", "-sTCP:LISTEN", "-n", "-P", "-F", "n"]
        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = Pipe()
        guard (try? p.run()) != nil else { return [] }
        p.waitUntilExit()
        let out = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        var ports = Set<Int>()
        for line in out.components(separatedBy: "\n") {
            guard line.hasPrefix("n") else { continue }
            let addr = String(line.dropFirst())
            let isLoopback = addr.hasPrefix("127.0.0.1:")
                || addr.hasPrefix("*:")
                || addr.hasPrefix("[::1]:")
                || addr.hasPrefix("::1:")
                || addr.hasPrefix("*.")
            guard isLoopback else { continue }
            let portStr = addr.components(separatedBy: ":").last?
                .trimmingCharacters(in: CharacterSet(charactersIn: "[]"))
            if let portStr, let port = Int(portStr), port > 1024 {
                ports.insert(port)
            }
        }
        return ports
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
