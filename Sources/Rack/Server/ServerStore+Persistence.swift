import AppKit
import Foundation

extension ServerStore {
  func load() {
    migrateDefaultsIfNeeded()
    guard let json = RackCore.shared.command(#"{"type":"servers.snapshot"}"#),
      let data = json.data(using: .utf8),
      let reply = try? decoder.decode(CoreServersReply.self, from: data)
    else {
      servers = []
      return
    }

    applyCoreConfiguration(reply.payload)
  }

  func migrateDefaultsIfNeeded() {
    let defaults = UserDefaults.standard
    if defaults.object(forKey: "terminalApp") == nil,
      let legacyDefaults = defaults.persistentDomain(forName: AppPaths.legacyDefaultsBundleID),
      let terminalApp = legacyDefaults["terminalApp"] as? String
    {
      defaults.set(terminalApp, forKey: "terminalApp")
    }
  }

  func save() {
    do {
      let command = CoreCommand(
        type: "servers.save",
        payload: PersistedConfiguration(servers: servers)
      )
      let data = try encoder.encode(command)
      guard let json = String(data: data, encoding: .utf8),
        let response = RackCore.shared.command(json),
        !response.contains(#""type":"error""#)
      else {
        NSSound.beep()
        return
      }
    } catch {
      NSSound.beep()
    }
  }
}
