import Foundation

func openLogInTerminal(path: String, id: String) {
  let appName = UserDefaults.standard.string(forKey: "terminalApp") ?? "Ghostty"
  let tailCommand = "tail -n 200 -f '\(shellEscape(path))'"

  switch appName.lowercased() {
  case "ghostty":
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
          input text "\(appleScriptEscape(tailCommand))" to term
          send key "enter" to term
      end tell
      """)
  case "terminal":
    runAppleScript(
      """
      tell application "Terminal"
          do script "\(appleScriptEscape(tailCommand))"
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
              write text "\(appleScriptEscape(tailCommand))"
          end tell
      end tell
      """)
  case "warp":
    openCommandFile(app: "Warp", id: id, command: tailCommand)
  default:
    openCommandFile(app: appName, id: id, command: tailCommand)
  }
}

private func runAppleScript(_ source: String) {
  run("/usr/bin/osascript", args: ["-e", source])
}

private func openCommandFile(app: String, id: String, command: String) {
  let url = URL(fileURLWithPath: NSTemporaryDirectory())
    .appending(path: "rack-\(safeFileName(id)).command")
  let content = "#!/bin/sh\n\(command)\n"
  guard (try? content.write(to: url, atomically: true, encoding: .utf8)) != nil else { return }
  try? FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
  run("/usr/bin/open", args: ["-a", app, url.path])
}

private func run(_ executable: String, args: [String]) {
  let process = Process()
  process.executableURL = URL(fileURLWithPath: executable)
  process.arguments = args
  try? process.run()
}

private func shellEscape(_ value: String) -> String {
  value.replacingOccurrences(of: "'", with: "'\\''")
}

private func appleScriptEscape(_ value: String) -> String {
  value
    .replacingOccurrences(of: "\\", with: "\\\\")
    .replacingOccurrences(of: "\"", with: "\\\"")
}

private func safeFileName(_ value: String) -> String {
  String(
    value.map { char in
      char.isLetter || char.isNumber || char == "-" || char == "_" ? char : "_"
    })
}
