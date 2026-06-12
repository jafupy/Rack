import AppKit
import Foundation

extension ServerStore {
  func openInTerminal(id: ServerConfiguration.ID) {
    guard let logURL = logFilePaths[id] else { return }
    let logPath = logURL.path
    let appName = UserDefaults.standard.string(forKey: "terminalApp") ?? "Ghostty"

    // Single-quoted shell-safe path
    let safePath = logPath.replacingOccurrences(of: "'", with: "'\\''")
    let tailCmd = "tail -n 200 -f '\(safePath)'"

    func run(_ executable: String, args: [String]) {
      let p = Process()
      p.executableURL = URL(fileURLWithPath: executable)
      p.arguments = args
      try? p.run()
    }

    func runAppleScript(_ source: String) {
      run("/usr/bin/osascript", args: ["-e", source])
    }

    func escapeAppleScriptString(_ value: String) -> String {
      value
        .replacingOccurrences(of: "\\", with: "\\\\")
        .replacingOccurrences(of: "\"", with: "\\\"")
    }

    // Write a temp .command file (executable shell script) for apps that support it
    func commandFileURL() -> URL? {
      let url = URL(fileURLWithPath: NSTemporaryDirectory())
        .appending(path: "\(AppPaths.commandFilePrefix)-\(id.uuidString).command")
      let content = "#!/bin/sh\n\(tailCmd)\n"
      guard (try? content.write(to: url, atomically: true, encoding: .utf8)) != nil else {
        return nil
      }
      try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
      return url
    }

    switch appName.lowercased() {
    case "ghostty":
      let escapedTailCmd = escapeAppleScriptString(tailCmd)
      runAppleScript(
        """
        tell application "Ghostty"
            activate
            set cfg to new surface configuration
            if (count of windows) = 0 then
                set win to new window with configuration cfg
                set term to focused terminal of selected tab of win
            else
                set win to front window
                set newTab to new tab in win with configuration cfg
                set term to focused terminal of newTab
            end if
            input text "\(escapedTailCmd)" to term
            send key "enter" to term
        end tell
        """)

    case "terminal":
      runAppleScript(
        """
        tell application "Terminal"
            do script "\(escapeAppleScriptString(tailCmd))"
            activate
        end tell
        """)

    case "iterm", "iterm2":
      runAppleScript(
        """
        tell application "iTerm2"
            activate
            set w to (create window with default profile)
            tell current session of w
                write text "\(escapeAppleScriptString(tailCmd))"
            end tell
        end tell
        """)

    case "warp":
      if let url = commandFileURL() {
        run("/usr/bin/open", args: ["-a", "Warp", url.path])
      }

    default:
      // Generic: open a .command file with the named app
      if let url = commandFileURL() {
        run("/usr/bin/open", args: ["-a", appName, url.path])
      }
    }
  }
}
