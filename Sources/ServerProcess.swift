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

    init(
        configuration: ServerConfiguration,
        outputHandler: @escaping @MainActor (String) -> Void,
        exitHandler: @escaping @MainActor (Int32) -> Void
    ) {
        self.configuration = configuration
        self.outputHandler = outputHandler
        self.exitHandler = exitHandler
    }

    func start() throws {
        let commandLine = shellEscapedCommandLine()
        process.executableURL = URL(fileURLWithPath: "/bin/zsh")
        process.arguments = ["-i", "-l", "-c", commandLine]

        let workingDirectory = normalizedPath(configuration.workingDirectory)
        if !workingDirectory.isEmpty {
            process.currentDirectoryURL = URL(filePath: workingDirectory, directoryHint: .isDirectory)
        }

        var environment = ProcessInfo.processInfo.environment
        // Force ANSI color output even though stdout is a pipe, not a TTY
        environment["TERM"] = "xterm-256color"
        environment["COLORTERM"] = "truecolor"
        environment["FORCE_COLOR"] = "1"
        environment["CLICOLOR_FORCE"] = "1"
        for variable in configuration.environment where !variable.key.isEmpty {
            environment[variable.key] = variable.value
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

        appendLog("Launching: \(commandLine)\n")
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

    private func shellEscapedCommandLine() -> String {
        let command = ([configuration.command] + configuration.parsedArguments)
            .map(shellEscape)
            .joined(separator: " ")
        return "clear && \(command)"
    }

    private func shellEscape(_ string: String) -> String {
        guard !string.isEmpty else { return "''" }
        return "'\(string.replacingOccurrences(of: "'", with: "'\\''"))'"
    }
}
