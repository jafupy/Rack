import AppKit
import SwiftUI

@MainActor
struct ServerSettingsPanel: View {
  @EnvironmentObject private var store: ServerStore
  @Binding var server: ServerConfiguration
  @State private var showingDeleteConfirmation = false

  private var status: ServerStatus { store.status(for: server.id) }
  private var isRunning: Bool { status.isRunning }
  private var hasLog: Bool { store.logFilePath(for: server.id) != nil }

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      header
      ServerConfigurationForm(server: $server)
      outputSection
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(Color.primary.opacity(0.08))
    }
    .confirmationDialog("Delete Server?", isPresented: $showingDeleteConfirmation, titleVisibility: .visible) {
      Button("Delete Server", role: .destructive) {
        store.selectedServerID = server.id
        store.deleteSelectedServer()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("This removes \(server.name.isEmpty ? "this server" : server.name) from Rack.")
    }
  }

  private var header: some View {
    HStack(alignment: .top, spacing: 12) {
      VStack(alignment: .leading, spacing: 5) {
        HStack(spacing: 8) {
          Circle()
            .fill(statusColor)
            .frame(width: 9, height: 9)
          Text(server.name.isEmpty ? "Unnamed Server" : server.name)
            .font(.system(size: 17, weight: .semibold))
            .lineLimit(1)
          Text(statusLabel)
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(statusColor)
        }

        HStack(spacing: 6) {
          Text(server.localURL.isEmpty ? "No route yet" : server.localURL)
            .font(.system(size: 12, design: .monospaced))
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.middle)

          Button {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(server.localURL, forType: .string)
          } label: {
            Image(systemName: "doc.on.doc")
          }
          .buttonStyle(.borderless)
          .foregroundStyle(.secondary)
          .disabled(server.localURL.isEmpty)
          .help("Copy local URL")
        }
      }

      Spacer()

      HStack(spacing: 8) {
        Button {
          store.restartServer(id: server.id)
        } label: {
          Label("Restart", systemImage: "arrow.clockwise")
        }
        .disabled(!isRunning)

        Button {
          if isRunning {
            store.stopServer(id: server.id)
          } else {
            store.startServer(id: server.id)
          }
        } label: {
          Label(isRunning ? "Stop" : "Start", systemImage: isRunning ? "stop.fill" : "play.fill")
        }
        .disabled(!isRunning && server.command.isEmpty)
        .buttonStyle(.borderedProminent)
        .tint(isRunning ? .red : .accentColor)

        actionsMenu
      }
    }
  }

  private var actionsMenu: some View {
    Menu {
      Button {
        store.selectedServerID = server.id
        store.duplicateSelectedServer()
      } label: {
        Label("Duplicate", systemImage: "plus.square.on.square")
      }

      Button {
        store.openInTerminal(id: server.id)
      } label: {
        Label("Open Logs", systemImage: "terminal")
      }
      .disabled(!hasLog)

      Divider()

      Button(role: .destructive) {
        showingDeleteConfirmation = true
      } label: {
        Label("Delete", systemImage: "trash")
      }
    } label: {
      Image(systemName: "ellipsis.circle")
    }
    .menuStyle(.borderlessButton)
    .help("Server actions")
  }

  private var outputSection: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        Label("Recent Output", systemImage: "text.alignleft")
          .font(.system(size: 11, weight: .semibold))
          .foregroundStyle(.secondary)
          .textCase(.uppercase)
        Spacer()
        Button {
          store.openInTerminal(id: server.id)
        } label: {
          Label("Open Full Logs", systemImage: "terminal")
            .font(.system(size: 11))
        }
        .disabled(!hasLog)
      }

      outputLog
    }
  }

  private var outputLog: some View {
    ScrollView {
      Group {
        if store.log(for: server.id).isEmpty {
          Text("No output yet.")
            .font(.system(size: 12, design: .monospaced))
            .foregroundStyle(.secondary)
        } else {
          Text(ansiAttributedString(store.log(for: server.id), fontSize: 12))
            .textSelection(.enabled)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(12)
    }
    .frame(height: 180)
    .background(poimandresBg, in: RoundedRectangle(cornerRadius: 8))
  }

  private var statusLabel: String {
    switch status {
    case .stopped: return "Stopped"
    case .starting: return "Starting"
    case .running: return "Running"
    case .failed: return "Failed"
    }
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
