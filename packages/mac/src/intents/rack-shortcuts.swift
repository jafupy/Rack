import AppIntents

struct RackShortcuts: AppShortcutsProvider {
  static let shortcutTileColor: ShortcutTileColor = .blue

  static var appShortcuts: [AppShortcut] {
    AppShortcut(
      intent: StartRackServiceIntent(),
      phrases: ["Start \(\.$service) in \(.applicationName)"],
      shortTitle: "Start Service",
      systemImageName: "play.fill"
    )
    AppShortcut(
      intent: StopRackServiceIntent(),
      phrases: ["Stop \(\.$service) in \(.applicationName)"],
      shortTitle: "Stop Service",
      systemImageName: "stop.fill"
    )
    AppShortcut(
      intent: RestartRackServiceIntent(),
      phrases: ["Restart \(\.$service) in \(.applicationName)"],
      shortTitle: "Restart Service",
      systemImageName: "arrow.clockwise"
    )
    AppShortcut(
      intent: StopAllRackServicesIntent(),
      phrases: ["Stop all services in \(.applicationName)"],
      shortTitle: "Stop All",
      systemImageName: "stop.circle"
    )
    AppShortcut(
      intent: ReloadRackHooksIntent(),
      phrases: ["Reload hooks in \(.applicationName)"],
      shortTitle: "Reload Hooks",
      systemImageName: "point.3.connected.trianglepath.dotted"
    )
  }
}
