import SwiftUI

@MainActor
public struct MenuBarContentView: View {
  @EnvironmentObject private var store: ServerStore
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false
  @State private var functionsExpanded = false
  var openSettings: (() -> Void)?

  public init(openSettings: (() -> Void)? = nil) {
    self.openSettings = openSettings
  }

  private var runningCount: Int {
    store.servers.filter { store.status(for: $0.id).isRunning }.count
  }

  public var body: some View {
    VStack(spacing: 0) {
      serverList
      if !store.functions.isEmpty {
        Divider()
        functionList
      }
      Divider()
      footer
    }
    .frame(width: 340)
    .fixedSize(horizontal: false, vertical: true)
  }

  private var functionList: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        Button {
          functionsExpanded.toggle()
        } label: {
          HStack(spacing: 6) {
            Image(systemName: functionsExpanded ? "chevron.down" : "chevron.right")
              .font(.system(size: 9, weight: .semibold))
              .frame(width: 10)
            Label("Functions", systemImage: "function")
          }
          .font(.system(size: 11, weight: .semibold))
          .foregroundStyle(.secondary)
        }
        .buttonStyle(.plain)

        Spacer()

        Button {
          store.reloadFunctions()
        } label: {
          Image(systemName: "arrow.clockwise")
        }
        .buttonStyle(.plain)
        .foregroundStyle(.secondary)
      }

      if functionsExpanded {
        ForEach(store.functions.prefix(4)) { function in
          FunctionMenuRow(function: function)
        }
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
        Button {
          showSettings()
        } label: {
          Text("Add a Server")
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
      }
      .frame(maxWidth: .infinity)
      .padding(.vertical, 30)
    } else {
      if store.servers.count > 4 {
        ScrollView {
          serverRows
        }
        .frame(maxHeight: 360)
      } else {
        serverRows
      }
    }
  }

  private var serverRows: some View {
    LazyVStack(spacing: 0) {
      ForEach(Array(store.servers.enumerated()), id: \.element.id) { index, server in
        ServerMenuRow(server: server)
        if index < store.servers.count - 1 {
          Divider().padding(.leading, 36)
        }
      }
    }
  }

  private var footer: some View {
    HStack {
      Button {
        showSettings()
      } label: {
        Image(systemName: "gear")
      }
      .buttonStyle(.plain)
      .font(.system(size: 11))
      .foregroundStyle(.secondary)

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

  private func showSettings() {
    if let openSettings {
      openSettings()
    }
  }
}

@MainActor
private struct FunctionMenuRow: View {
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false

  let function: ServerStore.FunctionSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack {
        Text(function.name)
          .font(.system(size: 12, weight: .medium))
          .lineLimit(1)
        Spacer()
        if !function.errors.isEmpty {
          Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.orange)
        }
      }

      ForEach(function.routes.prefix(2)) { route in
        Text("\(route.method) \(functionRouteURL(route.path))")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(1)
      }

      ForEach(function.crons.prefix(2)) { cron in
        Text("\(cron.schedule) -> \(cron.function)")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.tertiary)
          .lineLimit(1)
      }
    }
  }

  private func functionRouteURL(_ path: String) -> String {
    if standardPortsEnabled {
      return "rack.local\(path)"
    }
    return "localhost:\(ProxyServer.boundPort)\(path)"
  }
}

// MARK: - Server Row

@MainActor
private struct ServerMenuRow: View {
  @EnvironmentObject private var store: ServerStore
  let server: ServerConfiguration

  private var status: ServerStatus { store.status(for: server.id) }
  private var isRunning: Bool { status.isRunning }
  private var isStarting: Bool { status == .starting }
  private var hasLog: Bool { store.logFilePath(for: server.id) != nil }

  private var commandLabel: String {
    [server.command, server.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

  /// Last 3 non-empty visible lines, as an ANSI-attributed string.
  private var lastLinesAttributed: AttributedString? {
    let log = store.log(for: server.id)
    guard !log.isEmpty else { return nil }
    let lines = log.components(separatedBy: "\n").filter {
      !ANSIParser.strip($0).trimmingCharacters(in: .whitespaces).isEmpty
    }
    guard !lines.isEmpty else { return nil }
    return ANSIParser.attributedString(lines.suffix(3).joined(separator: "\n"), fontSize: 10)
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
          if isRunning, let url = URL(string: server.localURL) {
            Link(server.localURL, destination: url)
              .font(.system(size: 10, design: .monospaced))
              .foregroundStyle(.opacity(0.8))
              .lineLimit(1)
          } else if !commandLabel.isEmpty {
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
          if isRunning || isStarting {
            store.stopServer(id: server.id)
          } else {
            store.startServer(id: server.id)
          }
        } label: {
          if isStarting {
            ProgressView()
              .progressViewStyle(.circular)
              .controlSize(.mini)
              .frame(width: 26, height: 26)
              .background(.orange.opacity(0.1), in: Circle())
          } else {
            Image(systemName: isRunning ? "stop.fill" : "play.fill")
              .font(.system(size: 10, weight: .semibold))
              .foregroundStyle(isRunning ? Color.red : Color.green)
              .frame(width: 26, height: 26)
              .background(
                (isRunning ? Color.red : Color.green).opacity(0.1),
                in: Circle()
              )
          }
        }
        .buttonStyle(.plain)
        .disabled(server.command.isEmpty && !isRunning && !isStarting)
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
