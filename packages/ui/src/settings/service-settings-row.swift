import SwiftUI

struct ServiceSettingsRow: View {
  let service: ServiceConfiguration
  let onEdit: () -> Void
  let onDelete: () -> Void

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      Image(systemName: statusImage)
        .foregroundStyle(statusColor)
        .frame(width: 20)

      VStack(alignment: .leading, spacing: 4) {
        Text(service.name.isEmpty ? "Unnamed service" : service.name)
          .font(.headline)
        Text(commandLabel.isEmpty ? "No command configured" : commandLabel)
          .font(.system(.caption, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(2)
        Text(service.localURL)
          .font(.caption)
          .foregroundStyle(.tertiary)
        if !service.workingDir.isEmpty {
          Text(service.workingDir)
            .font(.caption)
            .foregroundStyle(.tertiary)
            .lineLimit(1)
        }
      }

      Spacer()

      VStack(alignment: .trailing, spacing: 8) {
        Text(statusLabel)
          .font(.caption.weight(.medium))
          .foregroundStyle(statusColor)

        HStack(spacing: 8) {
          Button("Edit", action: onEdit)
          Button("Remove", role: .destructive, action: onDelete)
        }
        .font(.caption)
      }
    }
    .padding(.vertical, 12)
  }

  private var commandLabel: String {
    [service.command, service.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

  private var statusLabel: String {
    switch service.status {
    case .stopped: "Stopped"
    case .starting: "Starting"
    case .running: "Running"
    case .failed: "Failed"
    }
  }

  private var statusImage: String {
    switch service.status {
    case .stopped: "circle"
    case .starting: "clock"
    case .running: "play.circle.fill"
    case .failed: "exclamationmark.triangle.fill"
    }
  }

  private var statusColor: Color {
    switch service.status {
    case .stopped: .secondary
    case .starting: .orange
    case .running: .green
    case .failed: .red
    }
  }
}
