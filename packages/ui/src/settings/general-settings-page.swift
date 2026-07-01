import SwiftUI

struct GeneralSettingsPage: View {
  let runningCount: Int
  let totalServiceCount: Int
  let hookCount: Int
  let actions: GeneralSettingsActions

  @State private var launchAtLoginEnabled: Bool
  @State private var launchAtLoginMessage: String?
  @State private var terminalName: String
  @State private var terminalMessage: String?
  @State private var cliInstallMessage: String?
  private let configPath: String?

  init(
    runningCount: Int,
    totalServiceCount: Int,
    hookCount: Int,
    state: GeneralSettingsState,
    actions: GeneralSettingsActions
  ) {
    self.runningCount = runningCount
    self.totalServiceCount = totalServiceCount
    self.hookCount = hookCount
    self.actions = actions
    _launchAtLoginEnabled = State(initialValue: state.launchAtLoginEnabled)
    _launchAtLoginMessage = State(initialValue: state.launchAtLoginMessage)
    _terminalName = State(initialValue: state.terminalName)
    _cliInstallMessage = State(initialValue: state.cliInstallMessage)
    configPath = state.configPath
  }

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "General",
        subtitle: "Product preferences for Rack startup, terminal integration, and local tooling."
      )

      summaryCard
      launchAtLoginCard
      terminalCard
      configurationCard
      cliCard
      quitCard
    }
  }

  private var summaryCard: some View {
    SettingsCard {
      VStack(alignment: .leading, spacing: 14) {
        LabeledContent("Services", value: "\(runningCount) running of \(totalServiceCount)")
        LabeledContent("Hooks", value: "\(hookCount) discovered")
      }
    }
  }

  private var launchAtLoginCard: some View {
    SettingsCard {
      Toggle("Launch Rack at login", isOn: launchBinding)
      if let launchAtLoginMessage {
        Text(launchAtLoginMessage).font(.caption).foregroundStyle(.secondary)
      }
    }
  }

  private var terminalCard: some View {
    SettingsCard {
      VStack(alignment: .leading, spacing: 10) {
        Text("Terminal").font(.headline)
        Picker("Open logs in", selection: terminalBinding) {
          ForEach(terminalOptions, id: \.self) { terminal in
            Text(terminal).tag(terminal)
          }
        }
        .pickerStyle(.menu)
        if let terminalMessage {
          Text(terminalMessage).font(.caption).foregroundStyle(.secondary)
        }
      }
    }
  }

  private var configurationCard: some View {
    SettingsCard {
      VStack(alignment: .leading, spacing: 12) {
        Text("Configuration").font(.headline)
        Text(configPath ?? "Config path is unavailable in this environment.")
          .font(.callout.monospaced())
          .foregroundStyle(.secondary)
          .textSelection(.enabled)
        HStack {
          Button("Reveal in Finder") { actions.revealConfig() }
          Button("Open") { actions.openConfig() }
        }
        .disabled(configPath == nil)
      }
    }
  }

  private var cliCard: some View {
    SettingsCard {
      VStack(alignment: .leading, spacing: 12) {
        Text("Command Line Tool").font(.headline)
        Text("Install the bundled `rack` command to `~/.local/bin/rack`.")
          .foregroundStyle(.secondary)
        Button("Install CLI") {
          cliInstallMessage = actions.installCLI()
        }
        if let cliInstallMessage {
          Text(cliInstallMessage).font(.caption).foregroundStyle(.secondary)
        }
      }
    }
  }

  private var quitCard: some View {
    SettingsCard {
      HStack {
        VStack(alignment: .leading, spacing: 4) {
          Text("Quit Rack").font(.headline)
          Text("Stops running services and exits the menu bar app.")
            .foregroundStyle(.secondary)
        }
        Spacer()
        Button("Quit", role: .destructive) { actions.quit() }
      }
    }
  }

  private var terminalOptions: [String] {
    let defaults = ["Ghostty", "Terminal", "iTerm2", "Warp"]
    return defaults.contains(terminalName) ? defaults : [terminalName] + defaults
  }

  private var launchBinding: Binding<Bool> {
    Binding(
      get: { launchAtLoginEnabled },
      set: { enabled in
        launchAtLoginEnabled = enabled
        launchAtLoginMessage = actions.setLaunchAtLogin(enabled)
      }
    )
  }

  private var terminalBinding: Binding<String> {
    Binding(
      get: { terminalName },
      set: { selected in
        let previous = terminalName
        terminalName = selected
        do {
          try actions.setTerminalName(selected)
          terminalMessage = "Saved to Rack config."
        } catch {
          terminalName = previous
          terminalMessage = error.localizedDescription
        }
      }
    )
  }
}
