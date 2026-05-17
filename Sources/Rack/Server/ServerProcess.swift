import Foundation

@MainActor
final class ServerProcess {
    let configuration: ServerConfiguration
    let process = Process()
    private let outputPipe = Pipe()
    private let errorPipe = Pipe()
    private let outputHandler: @MainActor (String) -> Void
    private let exitHandler: @MainActor (Int32) -> Void
    private(set) var logLines: [String] = []
    private let socketPath: String
    private let port: Int

    init(
        configuration: ServerConfiguration,
        socketPath: String,
        port: Int,
        outputHandler: @escaping @MainActor (String) -> Void,
        exitHandler: @escaping @MainActor (Int32) -> Void
    ) {
        self.configuration = configuration
        self.socketPath = socketPath
        self.port = port
        self.outputHandler = outputHandler
        self.exitHandler = exitHandler
    }

    func start() throws {
        let bridgePath = ServerProcess.findRackBridge()

        if let bridgePath {
            process.executableURL = URL(fileURLWithPath: bridgePath)
            process.arguments = buildRackBridgeArguments()
        } else {
            process.executableURL = URL(fileURLWithPath: "/bin/zsh")
            process.arguments = ["-i", "-l", "-c", shellFallbackCommandLine()]
        }

        let workingDirectory = normalizedPath(configuration.workingDirectory)
        if !workingDirectory.isEmpty {
            process.currentDirectoryURL = URL(filePath: workingDirectory, directoryHint: .isDirectory)
        }

        var environment = ProcessInfo.processInfo.environment
        for (key, value) in Self.loginShellEnvironment() {
            environment[key] = value
        }
        environment["TERM"] = "xterm-256color"
        environment["COLORTERM"] = "truecolor"
        environment["FORCE_COLOR"] = "1"
        environment["CLICOLOR_FORCE"] = "1"
        for variable in configuration.environment where !variable.key.isEmpty {
            environment[variable.key] = variable.value
        }
        // Inject PORT/HOST when not using rack-bridge (rack-bridge sets these itself).
        if bridgePath == nil {
            environment["PORT"] = String(port)
            environment["HOST"] = "127.0.0.1"
        }

        process.environment = environment
        process.standardOutput = outputPipe
        process.standardError = errorPipe

        let appendChunk: @Sendable (String) -> Void = { [weak self] output in
            Task { @MainActor in
                self?.appendLog(output)
            }
        }

        let reader: (FileHandle) -> Void = { handle in
            handle.readabilityHandler = { pipe in
                let data = pipe.availableData
                guard !data.isEmpty, let output = String(data: data, encoding: .utf8) else {
                    return
                }
                appendChunk(output)
            }
        }

        reader(outputPipe.fileHandleForReading)
        reader(errorPipe.fileHandleForReading)

        process.terminationHandler = { [weak self] process in
            Task { @MainActor [weak self] in
                self?.stopReading()
                self?.exitHandler(process.terminationStatus)
            }
        }

        appendLog("Launching: \(launchDescription(bridgePath: bridgePath))\n")
        if !workingDirectory.isEmpty {
            appendLog("Working directory: \(workingDirectory)\n")
        }

        try process.run()
    }

    func stop() {
        stopReading()
        guard process.isRunning else { return }
        process.terminate()
    }

    // MARK: - rack-bridge resolution

    /// Returns the path to the rack-bridge binary, or nil if not found.
    static func findRackBridge() -> String? {
        if let override = ProcessInfo.processInfo.environment["RACK_BRIDGE_PATH"],
           FileManager.default.isExecutableFile(atPath: override) {
            return override
        }
        // Bundled inside the .app (production)
        if let url = Bundle.main.resourceURL?.appending(path: "rack-bridge"),
           FileManager.default.isExecutableFile(atPath: url.path) {
            return url.path
        }
        // Development: .build/rust/release/rack-bridge relative to CWD
        let devPath = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appending(path: ".build/rust/release/rack-bridge").path
        if FileManager.default.isExecutableFile(atPath: devPath) {
            return devPath
        }
        return nil
    }

    static func loginShellEnvironment() -> [String: String] {
        let shell = ProcessInfo.processInfo.environment["SHELL"].flatMap {
            FileManager.default.isExecutableFile(atPath: $0) ? $0 : nil
        } ?? "/bin/zsh"

        let process = Process()
        process.executableURL = URL(fileURLWithPath: shell)
        process.arguments = ["-l", "-c", "printenv -0"]

        let output = Pipe()
        process.standardOutput = output
        process.standardError = Pipe()

        guard (try? process.run()) != nil else { return [:] }
        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { return [:] }

        return data.split(separator: 0).reduce(into: [:]) { result, entry in
            guard let line = String(data: Data(entry), encoding: .utf8),
                  let separator = line.firstIndex(of: "=")
            else { return }
            let key = String(line[..<separator])
            let value = String(line[line.index(after: separator)...])
            result[key] = value
        }
    }

    // MARK: - Private

    private func buildRackBridgeArguments() -> [String] {
        return [
            "--socket", socketPath,
            "--port", String(port),
            "--"
        ] + innerCommandTokens()
    }

    private func shellFallbackCommandLine() -> String {
        "clear; exec " + innerCommandTokens().map(shellEscape).joined(separator: " ")
    }

    /// Command + user args + optional portFlag injection.
    private func innerCommandTokens() -> [String] {
        var tokens = splitWords(configuration.command) + configuration.parsedArguments
        if let flag = configuration.portFlag, !flag.isEmpty {
            tokens += [flag, String(port)]
        }
        return tokens
    }

    private func launchDescription(bridgePath: String?) -> String {
        if bridgePath != nil {
            return "rack-bridge --socket \(socketPath) --port \(port) -- \(innerCommandTokens().joined(separator: " "))"
        }
        return innerCommandTokens().joined(separator: " ")
    }

    private func stopReading() {
        outputPipe.fileHandleForReading.readabilityHandler = nil
        errorPipe.fileHandleForReading.readabilityHandler = nil
    }

    private func appendLog(_ chunk: String) {
        let lines = chunk
            .replacingOccurrences(of: "\r\n", with: "\n")
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map(String.init)

        for line in lines where !line.isEmpty {
            logLines.append(line)
        }

        if logLines.count > 250 {
            logLines.removeFirst(logLines.count - 250)
        }

        outputHandler(chunk)
    }

    private func normalizedPath(_ path: String) -> String {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "" }
        return NSString(string: trimmed).expandingTildeInPath
    }

    private func shellEscape(_ string: String) -> String {
        guard !string.isEmpty else { return "''" }
        return "'\(string.replacingOccurrences(of: "'", with: "'\\''"))'"
    }

    private func splitWords(_ string: String) -> [String] {
        string
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
    }
}
