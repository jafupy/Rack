import AppKit
import SwiftUI

@MainActor
struct GeneralSettingsPage: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false
  @AppStorage("terminalApp") private var terminalApp = "Ghostty"
  @State private var portForwardingError = false
  @State private var portForwardingErrorMessage: String?

  private let terminals = ["Ghostty", "Terminal", "iTerm2", "Warp"]

  var body: some View {
    SettingsPage(section: .general, subtitle: "Configure Rack startup, routing, terminal, and local data.") {
      Form {
        appSection
        networkSection
        terminalSection
        dataSection
      }
      .formStyle(.grouped)
    }
    .onAppear {
      launchAtLogin.refresh()
    }
  }

  private var appSection: some View {
    Section {
      Toggle(
        "Launch at login",
        isOn: Binding(
          get: { launchAtLogin.isEnabled },
          set: { launchAtLogin.setEnabled($0) }
        )
      )

      if let errorMessage = launchAtLogin.errorMessage {
        Label(errorMessage, systemImage: "exclamationmark.triangle.fill")
          .font(.caption)
          .foregroundStyle(.orange)
          .fixedSize(horizontal: false, vertical: true)
      }

      Button(role: .destructive) {
        store.stopAllServers()
        NSApplication.shared.terminate(nil)
      } label: {
        Label("Quit Rack", systemImage: "power")
      }
    } header: {
      Label("App", systemImage: "app.badge")
    }
  }

  private var networkSection: some View {
    Section {
      Toggle(
        "Use standard web ports",
        isOn: Binding(
          get: { standardPortsEnabled },
          set: { enabled in
            portForwardingError = false
            portForwardingErrorMessage = nil
            if enabled {
              standardPortsEnabled = ProxyServer.setupPortForwarding()
              portForwardingError = !standardPortsEnabled
              portForwardingErrorMessage = ProxyServer.lastPortForwardingError
            } else {
              ProxyServer.teardownPortForwarding()
              standardPortsEnabled = false
            }
          }
        )
      )

      LabeledContent("Server routes") {
        Text(standardPortsEnabled ? "http://name.localhost" : "http://name.localhost:\(ProxyServer.boundPort)")
          .font(.system(size: 12, design: .monospaced))
          .foregroundStyle(.secondary)
      }

      if standardPortsEnabled {
        LabeledContent("Secure routes") {
          Text("https://name.localhost")
            .font(.system(size: 12, design: .monospaced))
            .foregroundStyle(.secondary)
        }
      } else {
        LabeledContent("Current proxy port") {
          Text("\(ProxyServer.boundPort)")
            .font(.system(size: 12, weight: .semibold, design: .monospaced))
            .monospacedDigit()
        }
      }

      if portForwardingError {
        Label(
          portForwardingErrorMessage
            ?? "Port forwarding setup failed. Administrator approval may be required.",
          systemImage: "exclamationmark.triangle.fill"
        )
          .font(.caption)
          .foregroundStyle(.red)
          .fixedSize(horizontal: false, vertical: true)
      }

      if standardPortsEnabled {
        Label("Standard ports run a privileged local relay for ports 80 and 443.", systemImage: "lock.shield")
          .font(.caption)
          .foregroundStyle(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
    } header: {
      Label("Network", systemImage: "network")
    } footer: {
      Text(standardPortsEnabled ? "Rack routes local servers through standard HTTP and HTTPS ports." : "Rack routes local servers through the running proxy port.")
    }
  }

  private var terminalSection: some View {
    Section {
      Picker("Terminal app", selection: $terminalApp) {
        ForEach(terminals, id: \.self) { terminal in
          Text(terminal).tag(terminal)
        }
      }
    } header: {
      Label("Terminal", systemImage: "terminal")
    } footer: {
      Text("Used by the Open Logs action for each server.")
    }
  }

  private var dataSection: some View {
    Section {
      LabeledContent("Config file") {
        Text(store.configurationURL.path)
          .font(.system(size: 12, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(1)
          .truncationMode(.middle)
      }

      Button {
        store.revealConfigurationFile()
      } label: {
        Label("Reveal in Finder", systemImage: "doc.text.magnifyingglass")
      }
    } header: {
      Label("Data", systemImage: "folder")
    }
  }
}
