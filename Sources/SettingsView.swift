import AppKit
import SwiftUI

@MainActor
struct SettingsView: View {
    @EnvironmentObject private var store: ServerStore
    @AppStorage("terminalApp") private var terminalApp = "Ghostty"

    var body: some View {
        NavigationSplitView {
            sidebar
                .navigationSplitViewColumnWidth(min: 220, ideal: 250, max: 300)
        } detail: {
            if let selectedServer = store.selectedServer {
                ServerEditorView(server: selectedServer)
                    .environmentObject(store)
            } else {
                detailEmptyState
            }
        }
    }

    // MARK: - Sidebar

    private var sidebar: some View {
        VStack(spacing: 0) {
            sidebarHeader
            Divider()
            if store.servers.isEmpty {
                sidebarEmptyState
            } else {
                List(selection: $store.selectedServerID) {
                    ForEach(store.servers) { server in
                        ServerListRow(server: server).tag(server.id)
                    }
                    .onDelete(perform: store.deleteServers)
                }
                .listStyle(.sidebar)
            }
            Divider()
            sidebarFooter
        }
        .background(.background)
    }

    private var sidebarHeader: some View {
        HStack {
            Text("Servers")
                .font(.system(size: 15, weight: .semibold))
            Spacer()
            HStack(spacing: 2) {
                Button {
                    store.duplicateSelectedServer()
                } label: {
                    Image(systemName: "doc.on.doc").frame(width: 28, height: 28)
                }
                .disabled(store.selectedServer == nil)
                .help("Duplicate")

                Button {
                    store.deleteSelectedServer()
                } label: {
                    Image(systemName: "trash").frame(width: 28, height: 28)
                }
                .disabled(store.selectedServer == nil)
                .help("Delete")

                Button {
                    store.addServer()
                } label: {
                    Image(systemName: "plus").frame(width: 28, height: 28)
                }
                .help("Add Server")
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }

    private var sidebarEmptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "plus.circle.dashed")
                .font(.system(size: 36))
                .foregroundStyle(.quaternary)
            Text("No servers yet")
                .font(.headline)
                .foregroundStyle(.secondary)
            Button { store.addServer() } label: {
                Label("Add Server", systemImage: "plus")
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var sidebarFooter: some View {
        HStack(spacing: 6) {
            Image(systemName: "terminal")
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
            TextField("Terminal", text: $terminalApp)
                .font(.system(size: 11))
                .textFieldStyle(.roundedBorder)
                .help("Terminal app for opening logs. Supported: Ghostty, Terminal, iTerm2, Warp")
            Spacer(minLength: 0)
            Button {
                store.revealConfigurationFile()
            } label: {
                Image(systemName: "doc.text")
            }
            .buttonStyle(.borderless)
            .foregroundStyle(.tertiary)
            .help("Open config file")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
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

// MARK: - Server Editor

@MainActor
private struct ServerEditorView: View {
    @EnvironmentObject private var store: ServerStore
    @Binding var server: ServerConfiguration
    @FocusState private var focusedField: Field?

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
            Circle()
                .fill(statusColor)
                .frame(width: 9, height: 9)

            VStack(alignment: .leading, spacing: 1) {
                Text(server.name.isEmpty ? "New Server" : server.name)
                    .font(.system(size: 15, weight: .semibold))
                    .lineLimit(1)
                Text(store.status(for: server.id).label)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Button("Restart") {
                store.restartServer(id: server.id)
            }
            .disabled(!isRunning)
            .buttonStyle(.bordered)
            .controlSize(.regular)

            Button(isRunning ? "Stop" : "Start") {
                if isRunning { store.stopServer(id: server.id) }
                else { store.startServer(id: server.id) }
            }
            .disabled(!isRunning && server.command.isEmpty)
            .buttonStyle(.borderedProminent)
            .tint(isRunning ? .red : .accentColor)
            .controlSize(.regular)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 12)
        .background(.bar)
    }

    // MARK: Config Form

    private var configForm: some View {
        Form {
            Section("Identity") {
                TextField("Name", text: $server.name)
                    .focused($focusedField, equals: .name)
                Toggle("Auto-start when Rack. launches", isOn: $server.autoStart)
            }

            Section("Command") {
                LabeledContent("Executable") {
                    TextField("bun, npm, cargo…", text: $server.command)
                        .focused($focusedField, equals: .command)
                        .fontDesign(.monospaced)
                }
                LabeledContent("Arguments") {
                    TextField("run dev --port 3000", text: $server.arguments)
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
            }

            Section("Environment") {
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
            }
        }
        .formStyle(.grouped)
    }

    // MARK: Output

    private var outputSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Output")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                if !store.log(for: server.id).isEmpty {
                    Button {
                        store.openInTerminal(id: server.id)
                    } label: {
                        Label("Open in Terminal", systemImage: "terminal")
                            .font(.system(size: 11))
                    }
                    .buttonStyle(.borderless)
                    .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 20)

            outputTerminal
                .padding(.horizontal, 20)
                .padding(.bottom, 20)
        }
        .padding(.top, 4)
    }

    private var outputTerminal: some View {
        ScrollViewReader { proxy in
            ScrollView {
                Group {
                    if store.log(for: server.id).isEmpty {
                        Text("No output yet.")
                            .font(.system(size: 12, design: .monospaced))
                            .foregroundStyle(.white.opacity(0.3))
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
        .frame(minHeight: 220)
        .background(.black.opacity(0.88), in: RoundedRectangle(cornerRadius: 10))
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
