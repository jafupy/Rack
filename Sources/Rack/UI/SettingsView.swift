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
    .onChange(of: selection) { _, newValue in
      if case .server(let id) = newValue {
        store.selectedServerID = id
      } else if newValue != .functions {
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

        Label("Functions", systemImage: "function")
          .tag(SidebarItem.functions)

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

@MainActor
private struct FunctionsSettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @AppStorage("functionWorkerLimit") private var functionWorkerLimit = 4

  private var routeCount: Int {
    store.functions.reduce(0) { $0 + $1.routes.count }
  }

  private var cronCount: Int {
    store.functions.reduce(0) { $0 + $1.crons.count }
  }

  private var errorCount: Int {
    store.functions.reduce(0) { $0 + $1.errors.count }
  }

  var body: some View {
    VStack(spacing: 0) {
      functionsToolbar

      ScrollView {
        VStack(alignment: .leading, spacing: 18) {
          overview
          runtimePanel

          if store.functions.isEmpty {
            emptyState
          } else {
            VStack(alignment: .leading, spacing: 10) {
              Text("Installed Packages")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(.secondary)
                .textCase(.uppercase)
                .tracking(0.4)

              LazyVStack(spacing: 10) {
                ForEach(store.functions) { function in
                  FunctionPackagePanel(function: function)
                }
              }
            }
          }
        }
        .frame(maxWidth: 760)
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 26)
        .padding(.vertical, 22)
      }
    }
    .background(functionsBackground)
    .onAppear {
      store.reloadFunctions()
    }
  }

  private var functionsToolbar: some View {
    HStack(spacing: 12) {
      VStack(alignment: .leading, spacing: 3) {
        HStack(spacing: 8) {
          Image(systemName: "function")
            .font(.system(size: 14, weight: .semibold))
            .foregroundStyle(.blue)
          Text("Functions")
            .font(.system(size: 15, weight: .semibold))
        }
        Text("Rust WASI packages served from rack.local")
          .font(.system(size: 11))
          .foregroundStyle(.secondary)
      }
      Spacer()
      Button {
        revealFunctionsFolder()
      } label: {
        Image(systemName: "folder")
      }
      .buttonStyle(.bordered)
      .help("Reveal ~/.rack/functions")

      Button {
        store.reloadFunctions()
      } label: {
        Image(systemName: "arrow.clockwise")
      }
      .buttonStyle(.borderedProminent)
      .help("Reload functions")
    }
    .padding(.horizontal, 24)
    .padding(.vertical, 12)
    .background(.bar)
  }

  private var overview: some View {
    HStack(spacing: 10) {
      FunctionMetric(title: "Packages", value: "\(store.functions.count)", systemImage: "shippingbox")
      FunctionMetric(title: "Routes", value: "\(routeCount)", systemImage: "point.topleft.down.curvedto.point.bottomright.up")
      FunctionMetric(title: "Schedules", value: "\(cronCount)", systemImage: "clock.badge")
      FunctionMetric(
        title: "Issues",
        value: "\(errorCount)",
        systemImage: errorCount == 0 ? "checkmark.seal" : "exclamationmark.triangle",
        tint: errorCount == 0 ? .green : .orange
      )
    }
  }

  private var runtimePanel: some View {
    HStack(alignment: .center, spacing: 18) {
      ZStack {
        RoundedRectangle(cornerRadius: 8)
          .fill(.blue.opacity(0.12))
        Image(systemName: "cpu")
          .font(.system(size: 22, weight: .medium))
          .foregroundStyle(.blue)
      }
      .frame(width: 48, height: 48)

      VStack(alignment: .leading, spacing: 5) {
        Text("Runtime Workers")
          .font(.system(size: 13, weight: .semibold))
        Text("Maximum concurrent rack.local function invocations.")
          .font(.system(size: 11))
          .foregroundStyle(.secondary)
      }

      Spacer()

      Stepper(value: $functionWorkerLimit, in: 1...32) {
        Text("\(functionWorkerLimit)")
          .font(.system(size: 20, weight: .semibold, design: .rounded))
          .monospacedDigit()
          .frame(width: 38, alignment: .trailing)
      }
      .frame(width: 116)
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(.quaternary)
    }
  }

  private var emptyState: some View {
    VStack(spacing: 14) {
      Image(systemName: "curlybraces.square")
        .font(.system(size: 38, weight: .light))
        .foregroundStyle(.secondary)
        .frame(width: 72, height: 72)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))

      VStack(spacing: 5) {
        Text("No Functions Installed")
          .font(.system(size: 16, weight: .semibold))
        Text("Create one with `rack fn init`, then add it to Rack.")
          .font(.system(size: 12))
          .foregroundStyle(.secondary)
          .multilineTextAlignment(.center)
      }

      HStack(spacing: 8) {
        FunctionCommandPill("rack fn init my-functions")
        FunctionCommandPill("rack fn add")
      }
    }
    .frame(maxWidth: .infinity)
    .padding(.vertical, 46)
    .padding(.horizontal, 24)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(.quaternary)
    }
  }

  private var functionsBackground: some View {
    ZStack {
      Color(nsColor: .windowBackgroundColor)
      LinearGradient(
        colors: [
          Color.blue.opacity(0.08),
          Color.clear,
          Color.green.opacity(0.04)
        ],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
      )
    }
  }

  private func revealFunctionsFolder() {
    let url = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".rack/functions")
    try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    NSWorkspace.shared.activateFileViewerSelecting([url])
  }
}

private struct FunctionMetric: View {
  let title: String
  let value: String
  let systemImage: String
  var tint: Color = .blue

  var body: some View {
    HStack(spacing: 10) {
      Image(systemName: systemImage)
        .font(.system(size: 14, weight: .semibold))
        .foregroundStyle(tint)
        .frame(width: 28, height: 28)
        .background(tint.opacity(0.12), in: RoundedRectangle(cornerRadius: 7))

      VStack(alignment: .leading, spacing: 1) {
        Text(value)
          .font(.system(size: 18, weight: .semibold, design: .rounded))
          .monospacedDigit()
        Text(title)
          .font(.system(size: 10, weight: .medium))
          .foregroundStyle(.secondary)
      }
      Spacer(minLength: 0)
    }
    .padding(12)
    .frame(maxWidth: .infinity)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(.quaternary)
    }
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

private struct FunctionCommandPill: View {
  let command: String

  init(_ command: String) {
    self.command = command
  }

  var body: some View {
    Text(command)
      .font(.system(size: 11, weight: .medium, design: .monospaced))
      .foregroundStyle(.secondary)
      .lineLimit(1)
      .padding(.horizontal, 10)
      .padding(.vertical, 6)
      .background(.quaternary, in: Capsule())
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
            ? "Servers available at http://name.localhost, https://name.localhost, http://rack.local, and https://rack.local. Requires administrator once; persists across reboots."
            : "Servers available at http://name.localhost:\(ProxyServer.boundPort). Functions are available at http://localhost:\(ProxyServer.boundPort).")
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
