import AppKit
import SwiftUI

@MainActor
struct ServerConfigurationForm: View {
  @Binding var server: ServerConfiguration

  var body: some View {
    Form {
      Section {
        TextField("Name", text: $server.name)
        Toggle("Auto-start when Rack launches", isOn: $server.autoStart)
        LabeledContent("Custom domain") {
          TextField("leave blank to use name", text: $server.customDomain)
            .fontDesign(.monospaced)
        }
      }

      Section {
        LabeledContent("Executable") {
          TextField("bun, npm, cargo", text: $server.command)
            .fontDesign(.monospaced)
        }
        LabeledContent("Arguments") {
          TextField("run dev", text: $server.arguments)
            .fontDesign(.monospaced)
        }
        LabeledContent("Working directory") {
          HStack {
            TextField("~/projects/app", text: $server.workingDirectory)
              .fontDesign(.monospaced)
            Button {
              pickWorkingDirectory()
            } label: {
              Label("Choose", systemImage: "folder")
            }
            .controlSize(.small)
          }
        }
        LabeledContent("Fixed port") {
          TextField("automatic", text: fixedPort)
            .fontDesign(.monospaced)
            .frame(maxWidth: 90)
        }
      }

      Section {
        ForEach($server.environment) { $variable in
          HStack(spacing: 6) {
            TextField("KEY", text: $variable.key)
              .fontDesign(.monospaced)
              .frame(maxWidth: 160)
            Text("=")
              .foregroundStyle(.tertiary)
              .fontDesign(.monospaced)
            TextField("value", text: $variable.value)
              .fontDesign(.monospaced)
          }
        }
        .onDelete { offsets in
          server.environment.remove(atOffsets: offsets)
        }

        Button {
          server.environment.append(.init())
        } label: {
          Label("Add Environment Variable", systemImage: "plus")
        }
      }
    }
    .formStyle(.grouped)
  }

  private var fixedPort: Binding<String> {
    Binding(
      get: { server.port.map(String.init) ?? "" },
      set: { value in
        let digits = value.filter(\.isNumber)
        server.port = digits.isEmpty ? nil : Int(digits)
      }
    )
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
