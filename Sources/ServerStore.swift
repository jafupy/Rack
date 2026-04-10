import AppKit
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

        // Create a fresh temp log file for this run
        let tmpDir = URL(fileURLWithPath: NSTemporaryDirectory()).appending(path: AppPaths.temporaryDirectoryName)
        try? FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
        let logURL = tmpDir.appending(path: "\(id.uuidString).log")
        FileManager.default.createFile(atPath: logURL.path, contents: nil)
        logFilePaths[id] = logURL
        logFileHandles[id] = try? FileHandle(forWritingTo: logURL)

        let process = ServerProcess(
            configuration: config,
            outputHandler: { [weak self] output in
                guard let self else { return }
                self.logs[id, default: ""] += output
                // Append to log file
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
            }
        )

        do {
            try process.start()
            processes[id] = process
            statuses[id] = .running(pid: process.process.processIdentifier)
        } catch {
            statuses[id] = .failed(message: error.localizedDescription)
        }
    }

    func stopServer(id: ServerConfiguration.ID) {
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
