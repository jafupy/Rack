import SwiftUI

struct ServicesSettingsPage: View {
  let services: [ServiceConfiguration]

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "Services",
        subtitle:
          "View configured services. Create, edit, and delete controls are placeholders until CRUD APIs exist."
      )

      if services.isEmpty {
        PlaceholderCard(
          systemImage: "plus.app",
          title: "No services configured",
          message: "Use this page to add services once configuration mutation is implemented."
        )
      } else {
        SettingsCard {
          VStack(spacing: 0) {
            ForEach(Array(services.enumerated()), id: \.element.id) { index, service in
              ServiceSettingsRow(service: service)
              if index < services.count - 1 {
                Divider()
              }
            }
          }
        }
      }

      Button {
      } label: {
        Label("Add Service", systemImage: "plus")
      }
      .disabled(true)
      .help("Service creation is not available until config mutation APIs exist.")
    }
  }
}

private struct ServiceSettingsRow: View {
  let service: ServiceConfiguration

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
      }

      Spacer()

      Text(statusLabel)
        .font(.caption.weight(.medium))
        .foregroundStyle(statusColor)
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
