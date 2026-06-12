import AppKit
import SwiftUI

// MARK: - Server Editor

@MainActor
struct ServerEditorView: View {
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
        Text(
          UserDefaults.standard.bool(forKey: "standardPortsEnabled")
            ? "Custom domain sets the subdomain: e.g. \"api\" → api.localhost"
            : "Custom domain sets the subdomain: e.g. \"api\" → api.localhost:\(ProxyServer.boundPort)"
        )
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
        Text(
          "Set Port if your server ignores the PORT environment variable (e.g. Astro, some Vite configs). Leave blank to auto-assign."
        )
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
