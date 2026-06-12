import SwiftUI

// MARK: - Sidebar Row

@MainActor
struct ServerListRow: View {
  @EnvironmentObject private var store: ServerStore
  let server: ServerConfiguration

  private var commandLabel: String {
    [server.command, server.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

  var body: some View {
    HStack(spacing: 10) {
      ZStack(alignment: .bottomTrailing) {
        RoundedRectangle(cornerRadius: 6)
          .fill(Color.accentColor.gradient)
        Image(systemName: "server.rack")
          .font(.system(size: 12, weight: .semibold))
          .foregroundStyle(.white)
        Circle()
          .fill(statusColor)
          .frame(width: 8, height: 8)
          .overlay(Circle().stroke(.background, lineWidth: 1.5))
          .offset(x: 2, y: 2)
      }
      .frame(width: 24, height: 24)

      VStack(alignment: .leading, spacing: 2) {
        Text(server.name)
          .font(.system(size: 13, weight: .medium))
          .foregroundStyle(.primary)
          .lineLimit(1)
        Text(commandLabel.isEmpty ? "No command" : commandLabel)
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(1)
      }
    }
    .padding(.vertical, 2)
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
