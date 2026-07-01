import Foundation

struct CLIInstaller {
  func install() -> String {
    guard let source = Bundle.main.resourceURL?.appending(path: "rack-cli") else {
      return "CLI installer is not available in this build."
    }
    guard FileManager.default.isExecutableFile(atPath: source.path) else {
      return "Bundled CLI was not found in this build."
    }

    let destinationDirectory = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".local")
      .appending(path: "bin")
    let destination = destinationDirectory.appending(path: "rack")

    do {
      try FileManager.default.createDirectory(
        at: destinationDirectory,
        withIntermediateDirectories: true
      )

      if FileManager.default.fileExists(atPath: destination.path) {
        let values = try destination.resourceValues(forKeys: [.isSymbolicLinkKey])
        guard values.isSymbolicLink == true else {
          return "~/.local/bin/rack already exists and is not a symlink."
        }
        try FileManager.default.removeItem(at: destination)
      }

      try FileManager.default.createSymbolicLink(at: destination, withDestinationURL: source)
      return "Installed rack at ~/.local/bin/rack."
    } catch {
      return "Failed to install CLI: \(error.localizedDescription)"
    }
  }
}
