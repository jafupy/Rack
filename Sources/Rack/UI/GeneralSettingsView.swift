import SwiftUI

// MARK: - General Settings

@MainActor
struct GeneralSettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @AppStorage("terminalApp") private var terminalApp = "Ghostty"
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false
  @State private var portForwardingError = false

  private let terminals = ["Ghostty", "Terminal", "iTerm2", "Warp"]

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        SettingsPageHeader(
          title: "Overview",
          subtitle: "Control how Rack starts, routes local services, and opens server logs.",
          systemImage: "server.rack"
        )

        Form {
          Section {
            Toggle(
              "Launch at login",
              isOn: Binding(
                get: { launchAtLogin.isEnabled },
                set: { launchAtLogin.setEnabled($0) }
              ))

            if let errorMessage = launchAtLogin.errorMessage {
              Text(errorMessage)
                .font(.caption)
                .foregroundStyle(.orange)
                .fixedSize(horizontal: false, vertical: true)
            }
          } header: {
            Label("Application", systemImage: "app.badge")
          }

          Section {
            Toggle(
              "Use standard web ports",
              isOn: Binding(
                get: { standardPortsEnabled },
                set: { enable in
                  portForwardingError = false
                  if enable {
                    if ProxyServer.setupPortForwarding() {
                      standardPortsEnabled = true
                    } else {
                      portForwardingError = true
                    }
                  } else {
                    ProxyServer.teardownPortForwarding()
                    standardPortsEnabled = false
                  }
                }
              ))
            LabeledContent("Active routes") {
              Text(
                standardPortsEnabled
                  ? "http://name.localhost" : "http://name.localhost:\(ProxyServer.boundPort)"
              )
              .font(.system(size: 12, design: .monospaced))
              .foregroundStyle(.secondary)
            }
            if portForwardingError {
              Text("Setup failed. Administrator access is required.")
                .font(.caption)
                .foregroundStyle(.red)
            }
          } header: {
            Label("Network", systemImage: "network")
          } footer: {
            Text(
              standardPortsEnabled
                ? "Also enables https://name.localhost, http://rack.local, and https://rack.local. Requires administrator approval once and persists across reboots."
                : "Functions are available through the local proxy at http://localhost:\(ProxyServer.boundPort)."
            )
            .foregroundStyle(.secondary)
          }

          Section {
            Picker("Terminal App", selection: $terminalApp) {
              ForEach(terminals, id: \.self) { Text($0) }
            }
          } header: {
            Label("Terminal", systemImage: "terminal")
          } footer: {
            Text("Used when opening server output logs.")
          }

          Section {
            Button {
              store.revealConfigurationFile()
            } label: {
              Label("Reveal Config File in Finder", systemImage: "doc.text.magnifyingglass")
            }
          } header: {
            Label("Data", systemImage: "folder")
          }
        }
        .formStyle(.grouped)
      }
      .frame(maxWidth: 780)
      .frame(maxWidth: .infinity, alignment: .top)
      .padding(.horizontal, 28)
      .padding(.vertical, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
    .onAppear {
      launchAtLogin.refresh()
    }
  }
}
