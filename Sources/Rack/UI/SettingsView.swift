import AppKit
import SwiftUI

private enum SidebarItem: Hashable {
  case general
  case server(UUID)
}

@MainActor
struct SettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @State private var selection: SidebarItem? = .general

  private var selectedServerID: UUID? {
    if case .server(let id) = selection { return id } else { return nil }
  }

  var body: some View {
    NavigationSplitView {
      sidebar
        .navigationSplitViewColumnWidth(min: 220, ideal: 250, max: 300)
    } detail: {
      switch selection {
      case .general:
        GeneralSettingsView()
          .environmentObject(store)
          .environmentObject(launchAtLogin)
      case .server:
        if let selectedServer = store.selectedServer {
          ServerEditorView(server: selectedServer)
            .environmentObject(store)
        } else {
          detailEmptyState
        }
      case nil:
        detailEmptyState
      }
    }
    .onChange(of: selection) { _, newValue in
      if case .server(let id) = newValue {
        store.selectedServerID = id
      } else {
        store.selectedServerID = nil
      }
    }
  }

  // MARK: - Sidebar

  private var sidebar: some View {
    VStack(spacing: 0) {
      sidebarHeader
      Divider()
      List(selection: $selection) {
        Label("General", systemImage: "gear")
          .tag(SidebarItem.general)

        Section {
          if store.servers.isEmpty {
            sidebarEmptyState
          } else {
            ForEach(store.servers) { server in
              ServerListRow(server: server).tag(SidebarItem.server(server.id))
            }
            .onDelete(perform: store.deleteServers)
          }
        } header: {
          HStack {
            Text("Servers")
            Spacer()
            Button {
              store.addServer()
            } label: {
              Image(systemName: "plus")
                .font(.system(size: 11, weight: .medium))
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
          }.padding(.trailing, 12)
        }
      }
      .listStyle(.sidebar)
      .padding(.top, 10)
    }
    .background(.background)
  }

  private var sidebarHeader: some View {
    HStack {
      Text("Settings")
        .font(.system(size: 15, weight: .semibold))
      Spacer()
    }
    .padding(.horizontal, 14)
    .padding(.vertical, 10)
  }

  private var sidebarEmptyState: some View {
    VStack(spacing: 8) {
      Image(systemName: "plus.circle.dashed")
        .font(.system(size: 28))
        .foregroundStyle(.quaternary)
      Text("No servers yet")
        .font(.subheadline)
        .foregroundStyle(.secondary)
      Button {
        store.addServer()
      } label: {
        Label("Add Server", systemImage: "plus")
      }
      .buttonStyle(.borderedProminent)
      .controlSize(.small)
    }
    .frame(maxWidth: .infinity)
    .padding(.vertical, 16)
  }

  // MARK: - Detail Empty State

  private var detailEmptyState: some View {
    ContentUnavailableView {
      Label("No Server Selected", systemImage: "slider.horizontal.3")
    } description: {
      Text("Select a server to configure it, or add a new one.")
    } actions: {
      Button("Add Server") { store.addServer() }
        .buttonStyle(.borderedProminent)
    }
  }
}

// MARK: - General Settings

@MainActor
private struct GeneralSettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @AppStorage("terminalApp") private var terminalApp = "Ghostty"
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false
  @State private var portForwardingError = false

  private let terminals = ["Ghostty", "Terminal", "iTerm2", "Warp"]

  var body: some View {
    ScrollView {
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
          Toggle("Standard web ports (80, 443)", isOn: Binding(
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
          if portForwardingError {
            Text("Setup failed — administrator access is required.")
              .font(.caption)
              .foregroundStyle(.red)
          }
        } header: {
          Label("Network", systemImage: "network")
        } footer: {
          Text(standardPortsEnabled
            ? "Servers available at http://name.localhost and https://name.localhost. Requires administrator once; persists across reboots."
            : "Servers available at http://name.localhost:\(ProxyServer.boundPort).")
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
          Button("Reveal Config File in Finder") {
            store.revealConfigurationFile()
          }
        } header: {
          Label("Data", systemImage: "folder")
        }
      }
      .formStyle(.grouped)
      .frame(maxWidth: 700)
      .padding(.vertical, 8)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
    .onAppear {
      launchAtLogin.refresh()
    }
  }
}

// MARK: - Server Editor

@MainActor
private struct ServerEditorView: View {
  @EnvironmentObject private var store: ServerStore
  @Binding var server: ServerConfiguration
  @FocusState private var focusedField: Field?
  @State private var showingDeleteConfirmation = false
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false

  private enum Field { case name, command }

  private var isRunning: Bool { store.status(for: server.id).isRunning }

  var body: some View {
    VStack(spacing: 0) {
      toolbar
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          configForm
          outputSection
        }
        .frame(maxWidth: 700)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 8)
      }
    }
    .background(.windowBackground)
    .onAppear {
      focusedField = server.name == "New Server" ? .name : .command
    }
  }

  // MARK: Toolbar

  private var toolbar: some View {
    HStack(alignment: .center, spacing: 12) {
      VStack(alignment: .leading, spacing: 4) {
        Text(server.name.isEmpty ? "New Server" : server.name)
          .font(.system(size: 15, weight: .semibold))
          .lineLimit(1)
        statusBadge
      }

      Spacer()

      Button("Delete") {
        showingDeleteConfirmation = true
      }
      .buttonStyle(.bordered)
      .tint(.red)
      .controlSize(.regular)

      Button("Restart") {
        store.restartServer(id: server.id)
      }
      .disabled(!isRunning)
      .buttonStyle(.bordered)
      .controlSize(.regular)

      Button(isRunning ? "Stop" : "Start") {
        if isRunning { store.stopServer(id: server.id) } else { store.startServer(id: server.id) }
      }
      .disabled(!isRunning && server.command.isEmpty)
      .buttonStyle(.borderedProminent)
      .tint(isRunning ? .red : .accentColor)
      .controlSize(.regular)
    }
    .padding(.horizontal, 24)
    .padding(.vertical, 14)
    .background(.bar)
    .confirmationDialog(
      "Delete Server?",
      isPresented: $showingDeleteConfirmation,
      titleVisibility: .visible
    ) {
      Button("Delete Server", role: .destructive) {
        store.deleteSelectedServer()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("This removes \(server.name.isEmpty ? "this server" : server.name) from Rack.")
    }
  }

  // MARK: Config Form

  private var configForm: some View {
    Form {
      Section {
        TextField("Name", text: $server.name)
          .focused($focusedField, equals: .name)
        Toggle("Auto-start when Rack. launches", isOn: $server.autoStart)
        LabeledContent("Local URL") {
          HStack(spacing: 6) {
            if let url = URL(string: server.localURL) {
              Link(server.localURL, destination: url)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.blue)
                .lineLimit(1)
            } else {
              Text(server.localURL)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
            }
            Button {
              NSPasteboard.general.clearContents()
              NSPasteboard.general.setString(server.localURL, forType: .string)
            } label: {
              Image(systemName: "doc.on.doc")
                .font(.system(size: 11))
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.secondary)
            .help("Copy URL")
          }
        }
        LabeledContent("Custom Domain") {
          TextField("leave blank to use name", text: $server.customDomain)
            .fontDesign(.monospaced)
        }
      } header: {
        Label("Identity", systemImage: "tag")
      } footer: {
        Text(UserDefaults.standard.bool(forKey: "standardPortsEnabled")
          ? "Custom domain sets the subdomain: e.g. \"api\" → api.localhost"
          : "Custom domain sets the subdomain: e.g. \"api\" → api.localhost:\(ProxyServer.boundPort)")
          .foregroundStyle(.secondary)
      }

      Section {
        LabeledContent("Executable") {
          TextField("bun, npm, cargo…", text: $server.command)
            .focused($focusedField, equals: .command)
            .fontDesign(.monospaced)
        }
        LabeledContent("Arguments") {
          TextField("run dev", text: $server.arguments)
            .fontDesign(.monospaced)
        }
        LabeledContent("Directory") {
          HStack {
            TextField("~/projects/app", text: $server.workingDirectory)
              .fontDesign(.monospaced)
            Button("Browse…") { pickWorkingDirectory() }
              .buttonStyle(.bordered)
              .controlSize(.small)
          }
        }
        LabeledContent("Port") {
          TextField(
            "auto",
            text: Binding(
              get: { server.port.map(String.init) ?? "" },
              set: { server.port = Int($0.filter(\.isNumber)) }
            )
          )
          .fontDesign(.monospaced)
          .frame(maxWidth: 80)
        }
      } header: {
        Label("Command", systemImage: "chevron.right.square")
      } footer: {
        Text("Set Port if your server ignores the PORT environment variable (e.g. Astro, some Vite configs). Leave blank to auto-assign.")
          .foregroundStyle(.secondary)
      }

      Section {
        ForEach($server.environment) { $variable in
          HStack(spacing: 4) {
            TextField("KEY", text: $variable.key)
              .fontDesign(.monospaced)
              .frame(maxWidth: 140)
            Text("=").foregroundStyle(.tertiary).fontDesign(.monospaced)
            TextField("value", text: $variable.value)
              .fontDesign(.monospaced)
          }
        }
        .onDelete { server.environment.remove(atOffsets: $0) }

        Button {
          server.environment.append(.init())
        } label: {
          Label("Add Variable", systemImage: "plus")
        }
        .buttonStyle(.borderless)
      } header: {
        Label("Environment", systemImage: "key.horizontal")
      }
    }
    .formStyle(.grouped)
  }

  // MARK: Output

  private var outputSection: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        Label("Output", systemImage: "text.alignleft")
          .font(.system(size: 11, weight: .semibold))
          .foregroundStyle(.secondary)
          .textCase(.uppercase)
          .tracking(0.3)
        Spacer()
        if !store.log(for: server.id).isEmpty {
          Button {
            store.openInTerminal(id: server.id)
          } label: {
            Label("Open in Terminal", systemImage: "arrow.up.right.square")
              .font(.system(size: 11))
          }
          .buttonStyle(.borderless)
          .foregroundStyle(.secondary)
        }
      }
      .padding(.horizontal, 20)

      outputTerminal
        .padding(.horizontal, 20)
        .padding(.bottom, 24)
    }
    .padding(.top, 8)
  }

  private var outputTerminal: some View {
    ScrollViewReader { proxy in
      ScrollView {
        Group {
          if store.log(for: server.id).isEmpty {
            Text("No output yet.")
              .font(.system(size: 12, design: .monospaced))
              .foregroundStyle(Color(red: 0.463, green: 0.486, blue: 0.616))
          } else {
            Text(ansiAttributedString(store.log(for: server.id), fontSize: 12))
              .textSelection(.enabled)
              .id("bottom")
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
      }
      .onChange(of: store.log(for: server.id)) { _, _ in
        proxy.scrollTo("bottom", anchor: .bottom)
      }
    }
    .frame(height: 220)
    .background(poimandresBg, in: RoundedRectangle(cornerRadius: 10))
  }

  // MARK: Helpers

  private var statusBadge: some View {
    HStack(spacing: 4) {
      Circle().fill(statusColor).frame(width: 7, height: 7)
        .shadow(color: statusColor.opacity(0.6), radius: 3)
      Text(store.status(for: server.id).label)
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(statusColor)
    }
    .padding(.horizontal, 8)
    .padding(.vertical, 4)
    .background(.quaternary, in: Capsule())
  }

  private var statusColor: Color {
    switch store.status(for: server.id) {
    case .stopped: return .secondary
    case .starting: return .orange
    case .running: return .green
    case .failed: return .red
    }
  }

  private func pickWorkingDirectory() {
    let panel = NSOpenPanel()
    panel.title = "Choose Working Directory"
    panel.canChooseFiles = false
    panel.canChooseDirectories = true
    panel.allowsMultipleSelection = false
    panel.canCreateDirectories = true
    if !server.workingDirectory.isEmpty {
      panel.directoryURL = URL(fileURLWithPath: server.workingDirectory)
    }
    if panel.runModal() == .OK, let url = panel.url {
      server.workingDirectory = url.path
    }
  }
}

// MARK: - Sidebar Row

@MainActor
private struct ServerListRow: View {
  @EnvironmentObject private var store: ServerStore
  let server: ServerConfiguration

  private var commandLabel: String {
    [server.command, server.arguments].filter { !$0.isEmpty }.joined(separator: " ")
  }

  var body: some View {
    HStack(spacing: 10) {
      Circle()
        .fill(statusColor)
        .frame(width: 8, height: 8)
      VStack(alignment: .leading, spacing: 2) {
        Text(server.name)
          .font(.system(size: 13, weight: .medium))
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
