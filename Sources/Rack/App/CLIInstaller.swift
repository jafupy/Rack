import Foundation

enum CLIInstaller {
    static func installBundledCLI() {
        guard let bundledCLI = Bundle.main.url(forResource: "rack", withExtension: nil) else {
            return
        }

        let fileManager = FileManager.default
        let home = fileManager.homeDirectoryForCurrentUser
        let binDirectory = home.appending(path: ".local/bin")
        let linkURL = binDirectory.appending(path: "rack")

        do {
            try fileManager.createDirectory(at: binDirectory, withIntermediateDirectories: true)
            if fileManager.fileExists(atPath: linkURL.path) {
                let attributes = try fileManager.attributesOfItem(atPath: linkURL.path)
                guard attributes[.type] as? FileAttributeType == .typeSymbolicLink else {
                    print("RackCLI install skipped: \(linkURL.path) already exists")
                    return
                }
                try fileManager.removeItem(at: linkURL)
            }
            try fileManager.createSymbolicLink(at: linkURL, withDestinationURL: bundledCLI)
            try ensurePathEntry(binDirectory: binDirectory, home: home)
        } catch {
            print("RackCLI failed to install: \(error)")
        }
    }

    private static func ensurePathEntry(binDirectory: URL, home: URL) throws {
        let profileURL = home.appending(path: ".zprofile")
        let entry = #"export PATH="$HOME/.local/bin:$PATH""#

        let current = (try? String(contentsOf: profileURL, encoding: .utf8)) ?? ""
        guard !current.contains("$HOME/.local/bin"),
              !current.contains("\(binDirectory.path):$PATH") else {
            return
        }

        var updated = current
        if !updated.isEmpty, !updated.hasSuffix("\n") {
            updated += "\n"
        }
        updated += "\n# Added by Rack.app\n\(entry)\n"
        try updated.write(to: profileURL, atomically: true, encoding: .utf8)
    }
}
