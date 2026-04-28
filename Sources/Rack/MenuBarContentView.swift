import SwiftUI

@MainActor
struct MenuBarContentView: View {
    @EnvironmentObject private var store: ServerStore
    @Environment(\.openWindow) private var openWindow

    private var runningCount: Int {
        store.servers.filter { store.status(for: $0.id).isRunning }.count
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            serverList
            Divider()
            footer
        }
        .frame(width: 340)
    }

    private var header: some View {
        HStack(spacing: 8) {
            Text("Rack.")
                .font(.system(size: 13, weight: .semibold))
            Spacer()
            if runningCount > 0 {
                HStack(spacing: 5) {
                    Circle().fill(.green).frame(width: 6, height: 6)
                    Text("\(runningCount) running")
                        .font(.system(size: 11))
                        .foregroundStyle(.green)
                }
            } else if !store.servers.isEmpty {
                Text("All stopped")
                    .font(.system(size: 11))
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }

    @ViewBuilder
    private var serverList: some View {
        if store.servers.isEmpty {
            VStack(spacing: 10) {
                Image(systemName: "server.rack")
                    .font(.system(size: 28))
                    .foregroundStyle(.quaternary)
                Text("No Servers")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.secondary)
                Button("Add a Server") {
                    openWindow(id: "main")
                    NSApplication.shared.activate(ignoringOtherApps: true)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 30)
        } else {
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(Array(store.servers.enumerated()), id: \.element.id) { index, server in
                        ServerMenuRow(server: server)
                        if index < store.servers.count - 1 {
                            Divider().padding(.leading, 36)
                        }
                    }
                }
            }
            .frame(maxHeight: 440)
        }
    }

    private var footer: some View {
        HStack {
            Button {
                openWindow(id: "main")
                NSApplication.shared.activate(ignoringOtherApps: true)
            } label: {
                Image(systemName: "gear")
            }
            .buttonStyle(.plain)
            .font(.system(size: 11))
            .foregroundStyle(.secondary)

            Spacer()

            if runningCount > 0 {
                Button("Stop All") { store.stopAllServers() }
                    .buttonStyle(.plain)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }

            Button("Quit") {
                store.stopAllServers()
                NSApplication.shared.terminate(nil)
            }
            .buttonStyle(.plain)
            .font(.system(size: 11))
            .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }
}

// MARK: - Server Row

@MainActor
private struct ServerMenuRow: View {
    @EnvironmentObject private var store: ServerStore
    let server: ServerConfiguration

    private var isRunning: Bool { store.status(for: server.id).isRunning }
    private var hasLog: Bool { store.logFilePath(for: server.id) != nil }

    private var commandLabel: String {
        [server.command, server.arguments].filter { !$0.isEmpty }.joined(separator: " ")
    }

    /// Last 3 non-empty visible lines, as an ANSI-attributed string.
    private var lastLinesAttributed: AttributedString? {
        let log = store.log(for: server.id)
        guard !log.isEmpty else { return nil }
        let lines = log.components(separatedBy: "\n").filter {
            !stripANSI($0).trimmingCharacters(in: .whitespaces).isEmpty
        }
        guard !lines.isEmpty else { return nil }
        return ansiAttributedString(lines.suffix(3).joined(separator: "\n"), fontSize: 10)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Row header: dot + name/cmd + terminal btn + play/stop btn
            HStack(spacing: 10) {
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)

                VStack(alignment: .leading, spacing: 2) {
                    Text(server.name.isEmpty ? "Unnamed" : server.name)
                        .font(.system(size: 13, weight: .medium))
                        .lineLimit(1)
                    if !commandLabel.isEmpty {
                        Text(commandLabel)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(.tertiary)
                            .lineLimit(1)
                    }
                }

                Spacer()

                // Terminal button
                Button {
                    store.openInTerminal(id: server.id)
                } label: {
                    Image(systemName: "terminal")
                        .font(.system(size: 10))
                        .foregroundStyle(hasLog ? Color.secondary : Color.secondary.opacity(0.4))
                        .frame(width: 26, height: 26)
                        .background(.quaternary.opacity(hasLog ? 1 : 0.4), in: Circle())
                }
                .buttonStyle(.plain)
                .disabled(!hasLog)
                .help("Open in \(UserDefaults.standard.string(forKey: "terminalApp") ?? "Ghostty")")

                // Play / stop button
                Button {
                    if isRunning {
                        store.stopServer(id: server.id)
                    } else {
                        store.startServer(id: server.id)
                    }
                } label: {
                    Image(systemName: isRunning ? "stop.fill" : "play.fill")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(isRunning ? Color.red : Color.green)
                        .frame(width: 26, height: 26)
                        .background(
                            (isRunning ? Color.red : Color.green).opacity(0.1),
                            in: Circle()
                        )
                }
                .buttonStyle(.plain)
                .disabled(server.command.isEmpty && !isRunning)
            }
            .padding(.horizontal, 14)
            .padding(.top, 10)
            .padding(.bottom, lastLinesAttributed == nil ? 10 : 8)

            // ANSI log preview — tap to open in terminal
            if let attributed = lastLinesAttributed {
                Button {
                    store.openInTerminal(id: server.id)
                } label: {
                    Text(attributed)
                        .lineLimit(3)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 6)
                        .background(.black.opacity(0.85), in: RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain)
                .padding(.horizontal, 14)
                .padding(.bottom, 10)
            }
        }
    }

    private var statusColor: Color {
        switch store.status(for: server.id) {
        case .stopped: return .secondary
        case .starting: return .orange
        case .running: return .green
        case .failed: return .red
        }
    }
}
