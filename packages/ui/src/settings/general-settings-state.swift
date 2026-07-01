public struct GeneralSettingsState {
  public var launchAtLoginEnabled: Bool
  public var launchAtLoginMessage: String?
  public var terminalName: String
  public var configPath: String?
  public var cliInstallMessage: String?

  public init(
    launchAtLoginEnabled: Bool = false,
    launchAtLoginMessage: String? = nil,
    terminalName: String = "Ghostty",
    configPath: String? = nil,
    cliInstallMessage: String? = nil
  ) {
    self.launchAtLoginEnabled = launchAtLoginEnabled
    self.launchAtLoginMessage = launchAtLoginMessage
    self.terminalName = terminalName
    self.configPath = configPath
    self.cliInstallMessage = cliInstallMessage
  }
}

public struct GeneralSettingsActions {
  public var setLaunchAtLogin: (Bool) -> String?
  public var setTerminalName: (String) throws -> Void
  public var revealConfig: () -> Void
  public var openConfig: () -> Void
  public var installCLI: () -> String
  public var quit: () -> Void

  public init(
    setLaunchAtLogin: @escaping (Bool) -> String? = { _ in nil },
    setTerminalName: @escaping (String) throws -> Void = { _ in },
    revealConfig: @escaping () -> Void = {},
    openConfig: @escaping () -> Void = {},
    installCLI: @escaping () -> String = { "CLI installer is not available in this build." },
    quit: @escaping () -> Void = {}
  ) {
    self.setLaunchAtLogin = setLaunchAtLogin
    self.setTerminalName = setTerminalName
    self.revealConfig = revealConfig
    self.openConfig = openConfig
    self.installCLI = installCLI
    self.quit = quit
  }
}
