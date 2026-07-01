import SwiftUI

@MainActor
struct ServiceMenuRow: View {
  @EnvironmentObject private var model: RackViewModel
  let service: ServiceConfiguration

  private var status: ServiceStatus { model.status(for: service.id) }
  private var isActive: Bool { status.isActive }
  private var hasLog: Bool { model.logFilePath(for: service.id) != nil }
  private var commandLabel: String {
    [service.command, service.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

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
      header
        .padding(.horizontal, 14)
        .padding(.top, 10)
        .padding(.bottom, lastLinesAttributed == nil ? 10 : 8)

      if let attributed = lastLinesAttributed {
        logPreview(attributed)
      }
    }
  }

  private var header: some View {
    HStack(spacing: 10) {
      Circle()
        .fill(statusColor)
        .frame(width: 8, height: 8)

      serviceLabel
      Spacer()
      terminalButton
      startStopButton
    }
  }

  private var serviceLabel: some View {
    VStack(alignment: .leading, spacing: 2) {
      Text(service.name.isEmpty ? "Unnamed" : service.name)
        .font(.system(size: 13, weight: .medium))
        .lineLimit(1)
      secondaryLabel
    }
  }

  @ViewBuilder
  private var secondaryLabel: some View {
    if status.isRunning, let url = URL(string: service.localURL) {
      Link(service.localURL, destination: url)
        .font(.system(size: 10, design: .monospaced))
        .foregroundStyle(.opacity(0.8))
        .lineLimit(1)
        .contextMenu {
          Button("Copy URL") {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(service.localURL, forType: .string)
          }
          Button("Open URL") { NSWorkspace.shared.open(url) }
        }
    } else if !commandLabel.isEmpty {
      Text(commandLabel)
        .font(.system(size: 10, design: .monospaced))
        .foregroundStyle(.tertiary)
        .lineLimit(1)
    }
  }

  private var terminalButton: some View {
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
    .help("Open in \(model.terminalName())")
  }

  private var startStopButton: some View {
    Button {
      isActive ? model.stop(id: service.id) : model.start(id: service.id)
    } label: {
      if status == .starting {
        ProgressView()
          .progressViewStyle(.circular)
          .controlSize(.mini)
          .frame(width: 26, height: 26)
          .background(.orange.opacity(0.1), in: Circle())
      } else {
        Image(systemName: isActive ? "stop.fill" : "play.fill")
          .font(.system(size: 10, weight: .semibold))
          .foregroundStyle(isActive ? Color.red : Color.green)
          .frame(width: 26, height: 26)
          .background((isActive ? Color.red : Color.green).opacity(0.1), in: Circle())
      }
    }
    .buttonStyle(.plain)
    .disabled(service.command.isEmpty && !isActive)
  }

  private func logPreview(_ attributed: AttributedString) -> some View {
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

  private var statusColor: Color {
    switch status {
    case .stopped: return .secondary
    case .starting: return .orange
    case .running: return .green
    case .failed: return .red
    }
  }
}
