import Foundation

struct InferredProject: Sendable {
    let name: String
    let sanitizedName: String
    let localURL: String

    static let proxyPort = 1355
}

/// Infers the project name from git, package.json, or directory name.
/// Also detects git worktrees and prepends the branch as a subdomain.
enum NameInferrer {
    static func infer(at directory: URL) -> InferredProject {
        let base = baseName(at: directory)
        let branch = worktreeBranch(at: directory)

        let name: String
        if let branch {
            // "feature/auth" -> "auth", "fix-ui" -> "fix-ui"
            let segment = branch.components(separatedBy: "/").last ?? branch
            name = "\(sanitize(segment)).\(sanitize(base))"
        } else {
            name = sanitize(base)
        }

        let url = "http://\(name).localhost:\(InferredProject.proxyPort)"
        return InferredProject(name: name, sanitizedName: name, localURL: url)
    }

    // MARK: Private

    private static func baseName(at directory: URL) -> String {
        // 1. Git remote URL
        if let remote = shell("git -C \(directory.path.quoted) remote get-url origin 2>/dev/null"),
           !remote.isEmpty {
            // https://github.com/user/myapp.git -> myapp
            let stripped = remote.trimmingCharacters(in: .whitespacesAndNewlines)
                .components(separatedBy: "/").last?
                .replacingOccurrences(of: ".git", with: "") ?? ""
            if !stripped.isEmpty { return stripped }
        }

        // 2. package.json name
        let pkgURL = directory.appending(path: "package.json")
        if let data = try? Data(contentsOf: pkgURL),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let name = json["name"] as? String, !name.isEmpty {
            // Strip scope: @scope/myapp -> myapp
            return name.components(separatedBy: "/").last ?? name
        }

        // 3. Directory name
        return directory.lastPathComponent
    }

    /// Returns the branch name if the current directory is a linked git worktree (not the main one).
    private static func worktreeBranch(at directory: URL) -> String? {
        guard let raw = shell("git -C \(directory.path.quoted) worktree list --porcelain 2>/dev/null"),
              !raw.isEmpty
        else { return nil }

        // Parse worktree blocks
        var worktrees: [(path: String, branch: String?, isMain: Bool)] = []
        var current: [String: String] = [:]

        for line in (raw + "\n").components(separatedBy: "\n") {
            if line.isEmpty {
                if let path = current["worktree"] {
                    let branch = current["branch"].map {
                        $0.replacingOccurrences(of: "refs/heads/", with: "")
                    }
                    worktrees.append((path: path, branch: branch, isMain: current["bare"] == nil && worktrees.isEmpty))
                }
                current = [:]
            } else {
                let parts = line.split(separator: " ", maxSplits: 1)
                if parts.count == 2 {
                    current[String(parts[0])] = String(parts[1])
                } else if parts.count == 1 {
                    current[String(parts[0])] = ""
                }
            }
        }

        guard let first = worktrees.first else { return nil }

        // If we're in the main worktree, no prefix needed
        let currentPath = (try? directory.path.realpath()) ?? directory.path
        let mainPath = (try? first.path.realpath()) ?? first.path
        if currentPath == mainPath { return nil }

        // Find which worktree we're in
        let match = worktrees.first { wt in
            let wtPath = (try? wt.path.realpath()) ?? wt.path
            return currentPath.hasPrefix(wtPath)
        }

        return match?.branch
    }
}

// MARK: - Helpers

private func shell(_ command: String) -> String? {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/bin/sh")
    process.arguments = ["-c", command]
    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = Pipe()
    guard (try? process.run()) != nil else { return nil }
    process.waitUntilExit()
    return String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)
}

private func sanitize(_ name: String) -> String {
    name.lowercased()
        .replacingOccurrences(of: " ", with: "-")
        .components(separatedBy: .init(charactersIn: "^@#$%&*()[]{}|;:',.<>?/\\\""))
        .joined()
}

private extension String {
    var quoted: String { "'\(self.replacingOccurrences(of: "'", with: "'\\''"))'" }

    func realpath() throws -> String {
        var buf = [CChar](repeating: 0, count: Int(PATH_MAX))
        guard Darwin.realpath(self, &buf) != nil else {
            throw CocoaError(.fileNoSuchFile)
        }
        return String(cString: buf)
    }
}
