import AppKit
import SwiftUI

private enum SidebarItem: Hashable {
  case general
  case functions
  case server(UUID)
}

@MainActor
struct SettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @State private var selection: SidebarItem? = .general

  var body: some View {
    NavigationSplitView {
      sidebar
    } detail: {
      detail
    }
    .onChange(of: selection) { _, newValue in
      if case .server(let id) = newValue {
        store.selectedServerID = id
      } else if newValue != .functions {
        store.selectedServerID = nil
      }
    }
  }

  @ViewBuilder
  private var detail: some View {
    switch selection {
    case .general:
      GeneralSettingsView()
        .environmentObject(store)
        .environmentObject(launchAtLogin)
    case .functions:
      FunctionsSettingsView()
        .environmentObject(store)
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

  // MARK: - Sidebar

  private var sidebar: some View {
    List(selection: $selection) {
      Label("Overview", systemImage: "rectangle.grid.2x2")
        .tag(SidebarItem.general)

      Label("Functions", systemImage: "function")
        .tag(SidebarItem.functions)

      Section {
        if store.servers.isEmpty {
          Label("No servers", systemImage: "server.rack")
          .foregroundStyle(.secondary)
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
            selection = store.servers.last.map { .server($0.id) }
          } label: {
            Image(systemName: "plus")
          }
          .buttonStyle(.plain)
          .foregroundStyle(.secondary)
          .help("Add Server")
        }
        .padding(.trailing, 8)
      }
    }
    .listStyle(.sidebar)
    .navigationTitle("Rack. Settings")
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

@MainActor
private struct FunctionsSettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @AppStorage("functionWorkerLimit") private var functionWorkerLimit = 4

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        SettingsPageHeader(
          title: "Functions",
          subtitle: "Inspect local Rust WASI packages, routes, schedules, and worker capacity.",
          systemImage: "function"
        ) {
          Button {
            revealFunctionsFolder()
          } label: {
            Label("Reveal", systemImage: "folder")
          }

          Button {
            store.reloadFunctions()
          } label: {
            Label("Reload", systemImage: "arrow.clockwise")
          }
        }

        Form {
          Section {
            Stepper(value: $functionWorkerLimit, in: 1...32) {
              LabeledContent("Runtime Workers") {
                Text("\(functionWorkerLimit)")
                  .font(.system(size: 13, weight: .semibold, design: .monospaced))
                  .monospacedDigit()
              }
            }
          } footer: {
            Text("Maximum concurrent rack.local function invocations.")
          }
        }
        .formStyle(.grouped)

        if store.functions.isEmpty {
          FunctionEmptyState(revealFunctionsFolder: revealFunctionsFolder, reloadFunctions: store.reloadFunctions)
        } else {
          VStack(alignment: .leading, spacing: 10) {
            Text("Installed Packages")
              .font(.system(size: 11, weight: .semibold))
              .foregroundStyle(.secondary)
              .textCase(.uppercase)

            LazyVStack(spacing: 10) {
              ForEach(store.functions) { function in
                FunctionPackagePanel(function: function)
              }
            }
          }
        }
      }
      .frame(maxWidth: 820)
      .frame(maxWidth: .infinity, alignment: .top)
      .padding(.horizontal, 28)
      .padding(.vertical, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
    .onAppear {
      store.reloadFunctions()
    }
  }

  private func revealFunctionsFolder() {
    let url = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".rack/functions")
    try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    NSWorkspace.shared.activateFileViewerSelecting([url])
  }
}

private struct FunctionPackagePanel: View {
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false

  let function: ServerStore.FunctionSummary

  private var hasErrors: Bool {
    !function.errors.isEmpty
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .firstTextBaseline, spacing: 10) {
        VStack(alignment: .leading, spacing: 3) {
          HStack(spacing: 8) {
            Text(function.name)
              .font(.system(size: 15, weight: .semibold))
              .lineLimit(1)
            Text(function.version)
              .font(.system(size: 10, weight: .medium, design: .monospaced))
              .foregroundStyle(.secondary)
              .padding(.horizontal, 6)
              .padding(.vertical, 2)
              .background(.quaternary, in: Capsule())
          }
          Text(function.root)
            .font(.system(size: 10, design: .monospaced))
            .foregroundStyle(.tertiary)
            .lineLimit(1)
            .truncationMode(.middle)
        }

        Spacer()

        Label(hasErrors ? "Needs attention" : "Ready", systemImage: hasErrors ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
          .font(.system(size: 11, weight: .medium))
          .foregroundStyle(hasErrors ? .orange : .green)
      }

      if !function.routes.isEmpty {
        FunctionEndpointGroup(title: "Routes", systemImage: "arrow.triangle.branch", tint: .blue) {
          ForEach(function.routes) { route in
            FunctionEndpointRow(
              leading: route.method,
              main: functionRouteURL(route.path),
              trailing: route.function,
              tint: .blue
            )
          }
        }
      }

      if !function.crons.isEmpty {
        FunctionEndpointGroup(title: "Schedules", systemImage: "clock", tint: .green) {
          ForEach(function.crons) { cron in
            FunctionEndpointRow(
              leading: "cron",
              main: cron.schedule,
              trailing: cron.function,
              tint: .green
            )
          }
        }
      }

      if !function.errors.isEmpty {
        VStack(alignment: .leading, spacing: 6) {
          ForEach(function.errors, id: \.self) { error in
            Label(error, systemImage: "exclamationmark.triangle.fill")
              .font(.system(size: 11, weight: .medium))
              .foregroundStyle(.orange)
              .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
        .padding(10)
        .background(.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 8))
      }
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(hasErrors ? Color.orange.opacity(0.35) : Color.primary.opacity(0.08))
    }
  }

  private func functionRouteURL(_ path: String) -> String {
    if standardPortsEnabled {
      return "rack.local\(path)"
    }
    return "localhost:\(ProxyServer.boundPort)\(path)"
  }
}

private struct FunctionEndpointGroup<Content: View>: View {
  let title: String
  let systemImage: String
  let tint: Color
  @ViewBuilder var content: Content

  var body: some View {
    VStack(alignment: .leading, spacing: 7) {
      Label(title, systemImage: systemImage)
        .font(.system(size: 10, weight: .semibold))
        .foregroundStyle(tint)
        .textCase(.uppercase)
        .tracking(0.3)
      VStack(spacing: 6) {
        content
      }
    }
  }
}

private struct FunctionEndpointRow: View {
  let leading: String
  let main: String
  let trailing: String
  let tint: Color

  var body: some View {
    HStack(spacing: 10) {
      Text(leading.uppercased())
        .font(.system(size: 10, weight: .bold, design: .monospaced))
        .foregroundStyle(tint)
        .frame(width: 48, alignment: .leading)
      Text(main)
        .font(.system(size: 12, weight: .medium, design: .monospaced))
        .lineLimit(1)
        .truncationMode(.middle)
      Spacer(minLength: 8)
      Text(trailing)
        .font(.system(size: 11, design: .monospaced))
        .foregroundStyle(.secondary)
        .lineLimit(1)
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 7)
    .background(.quinary, in: RoundedRectangle(cornerRadius: 7))
  }
}

private struct SettingsPageHeader<Actions: View>: View {
  let title: String
  let subtitle: String
  let systemImage: String
  @ViewBuilder var actions: Actions

  init(
    title: String,
    subtitle: String,
    systemImage: String,
    @ViewBuilder actions: () -> Actions = { EmptyView() }
  ) {
    self.title = title
    self.subtitle = subtitle
    self.systemImage = systemImage
    self.actions = actions()
  }

  var body: some View {
    HStack(alignment: .center, spacing: 14) {
      ZStack {
        RoundedRectangle(cornerRadius: 8)
          .fill(.tertiary.opacity(0.35))
        Image(systemName: systemImage)
          .font(.system(size: 22, weight: .semibold))
          .foregroundStyle(.primary)
      }
      .frame(width: 52, height: 52)

      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .font(.system(size: 28, weight: .semibold))
          .lineLimit(1)
        Text(subtitle)
          .font(.system(size: 13))
          .foregroundStyle(.secondary)
          .lineLimit(2)
      }

      Spacer(minLength: 16)

      HStack(spacing: 8) {
        actions
      }
      .controlSize(.regular)
    }
  }
}

private struct FunctionEmptyState: View {
  let revealFunctionsFolder: () -> Void
  let reloadFunctions: () -> Void

  var body: some View {
    ContentUnavailableView {
      Label("No Functions Installed", systemImage: "curlybraces.square")
    } description: {
      Text("Create a package with `rack fn init`, then add it to Rack.")
    } actions: {
      HStack {
        Button {
          revealFunctionsFolder()
        } label: {
          Label("Reveal Folder", systemImage: "folder")
        }

        Button {
          reloadFunctions()
        } label: {
          Label("Reload", systemImage: "arrow.clockwise")
        }
      }
    }
    .frame(maxWidth: .infinity, minHeight: 180)
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
            Toggle("Use standard web ports", isOn: Binding(
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
              Text(standardPortsEnabled ? "http://name.localhost" : "http://name.localhost:\(ProxyServer.boundPort)")
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
            Text(standardPortsEnabled
              ? "Also enables https://name.localhost, http://rack.local, and https://rack.local. Requires administrator approval once and persists across reboots."
              : "Functions are available through the local proxy at http://localhost:\(ProxyServer.boundPort).")
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
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        SettingsPageHeader(
          title: server.name.isEmpty ? "New Server" : server.name,
          subtitle: server.localURL,
          systemImage: "server.rack"
        ) {
          Button(role: .destructive) {
            showingDeleteConfirmation = true
          } label: {
            Label("Delete", systemImage: "trash")
          }

          Button {
            store.restartServer(id: server.id)
          } label: {
            Label("Restart", systemImage: "arrow.clockwise")
          }
          .disabled(!isRunning)

          Button {
            if isRunning { store.stopServer(id: server.id) } else { store.startServer(id: server.id) }
          } label: {
            Label(isRunning ? "Stop" : "Start", systemImage: isRunning ? "stop.fill" : "play.fill")
          }
          .disabled(!isRunning && server.command.isEmpty)
          .buttonStyle(.borderedProminent)
          .tint(isRunning ? .red : .accentColor)
        }

        configForm
        outputSection
      }
      .frame(maxWidth: 780)
      .frame(maxWidth: .infinity, alignment: .top)
      .padding(.horizontal, 28)
      .padding(.vertical, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
    .onAppear {
      focusedField = server.name == "New Server" ? .name : .command
    }
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
