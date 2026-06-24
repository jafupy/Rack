import SwiftUI

@MainActor
public struct MenuBarContentView: View {
  @EnvironmentObject private var model: RackViewModel
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false
  @State private var hooksExpanded = false
  var openSettings: (() -> Void)?

  public init(openSettings: (() -> Void)? = nil) {
    self.openSettings = openSettings
  }

  private var runningCount: Int {
    model.services.filter { model.status(for: $0.id).isRunning }.count
  }

  public var body: some View {
    VStack(spacing: 0) {
      serviceList
      if !model.hooks.isEmpty {
        Divider()
        hookList
      }
      Divider()
      footer
    }
    .frame(width: 340)
    .fixedSize(horizontal: false, vertical: true)
  }

  private var hookList: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        Button {
          hooksExpanded.toggle()
        } label: {
          HStack(spacing: 6) {
            Image(systemName: hooksExpanded ? "chevron.down" : "chevron.right")
              .font(.system(size: 9, weight: .semibold))
              .frame(width: 10)
            Label("Hooks", systemImage: "point.3.connected.trianglepath.dotted")
          }
          .font(.system(size: 11, weight: .semibold))
          .foregroundStyle(.secondary)
        }
        .buttonStyle(.plain)

        Spacer()
      }

      if hooksExpanded {
        ForEach(model.hooks.prefix(4)) { hook in
          HookMenuRow(hook: hook)
        }
      }
    }
    .padding(.horizontal, 14)
    .padding(.vertical, 10)
  }

  @ViewBuilder
  private var serviceList: some View {
    if model.services.isEmpty {
      VStack(spacing: 10) {
        Image(systemName: "server.rack")
          .font(.system(size: 28))
          .foregroundStyle(.quaternary)
        Text("No Services")
          .font(.system(size: 13, weight: .medium))
          .foregroundStyle(.secondary)
        Button {
          showSettings()
        } label: {
          Text("Add a Service")
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
      }
      .frame(maxWidth: .infinity)
      .padding(.vertical, 30)
    } else {
      if model.services.count > 4 {
        ScrollView {
          serviceRows
        }
        .frame(maxHeight: 360)
      } else {
        serviceRows
      }
    }
  }

  private var serviceRows: some View {
    LazyVStack(spacing: 0) {
      ForEach(Array(model.services.enumerated()), id: \.element.id) { index, service in
        ServiceMenuRow(service: service)
        if index < model.services.count - 1 {
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
      } else if !model.services.isEmpty {
        Text("All stopped")
          .font(.system(size: 11))
          .foregroundStyle(.tertiary)
      }

      Spacer()

      if runningCount > 0 {
        Button("Stop All") { model.stopAll() }
          .buttonStyle(.plain)
          .font(.system(size: 11))
          .foregroundStyle(.secondary)
      }

      Button("Quit") {
        model.stopAll()
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
private struct HookMenuRow: View {
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false

  let hook: HookSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack {
        Text(hook.name)
          .font(.system(size: 12, weight: .medium))
          .lineLimit(1)
        Spacer()
        if !hook.errors.isEmpty {
          Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.orange)
        }
      }

      ForEach(hook.routes.prefix(2)) { route in
        Text("\(route.method) \(hookRouteURL(route.path))")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(1)
      }

      ForEach(hook.crons.prefix(2)) { cron in
        Text("\(cron.schedule) -> \(cron.hook)")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.tertiary)
          .lineLimit(1)
      }
    }
  }

  private func hookRouteURL(_ path: String) -> String {
    if standardPortsEnabled {
      return "rack.local\(path)"
    }
    return "localhost:\(RackProxy.fallbackPort)\(path)"
  }
}

// MARK: - Service Row

@MainActor
private struct ServiceMenuRow: View {
  @EnvironmentObject private var model: RackViewModel
  let service: ServiceConfiguration

  private var status: ServiceStatus { model.status(for: service.id) }
  private var isRunning: Bool { status.isRunning }
  private var isStarting: Bool { status == .starting }
  private var hasLog: Bool { model.logFilePath(for: service.id) != nil }

  private var commandLabel: String {
    [service.command, service.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

  /// Last 3 non-empty visible lines, as an ANSI-attributed string.
  private var lastLinesAttributed: AttributedString? {
    let log = model.log(for: service.id)
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
          Text(service.name.isEmpty ? "Unnamed" : service.name)
            .font(.system(size: 13, weight: .medium))
            .lineLimit(1)
          if isRunning, let url = URL(string: service.localURL) {
            Link(service.localURL, destination: url)
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
          model.openInTerminal(id: service.id)
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

        Button {
          model.restart(id: service.id)
        } label: {
          Image(systemName: "arrow.clockwise")
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(isRunning ? Color.secondary : Color.secondary.opacity(0.4))
            .frame(width: 26, height: 26)
            .background(.quaternary.opacity(isRunning ? 1 : 0.4), in: Circle())
        }
        .buttonStyle(.plain)
        .disabled(!isRunning)
        .help("Restart service")

        // Play / stop button
        Button {
          if isRunning || isStarting {
            model.stop(id: service.id)
          } else {
            model.start(id: service.id)
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
        .disabled(service.command.isEmpty && !isRunning && !isStarting)
      }
      .padding(.horizontal, 14)
      .padding(.top, 10)
      .padding(.bottom, lastLinesAttributed == nil ? 10 : 8)

      // ANSI log preview — tap to open in terminal
      if let attributed = lastLinesAttributed {
        Button {
          model.openInTerminal(id: service.id)
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
    switch model.status(for: service.id) {
    case .stopped: return .secondary
    case .starting: return .orange
    case .running: return .green
    case .failed: return .red
    }
  }
}
