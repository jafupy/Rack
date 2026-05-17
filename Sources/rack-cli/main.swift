import Foundation

// MARK: - IPC client

func send(_ message: [String: Any]) throws -> [String: Any]? {
    let socketPath = FileManager.default.homeDirectoryForCurrentUser
        .appending(path: ".config/rack/rack.sock").path

    let sock = socket(AF_UNIX, SOCK_STREAM, 0)
    guard sock >= 0 else {
        throw CLIError("Cannot create socket")
    }
    defer { close(sock) }

    var addr = sockaddr_un()
    addr.sun_family = sa_family_t(AF_UNIX)
    withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
        ptr.withMemoryRebound(to: CChar.self, capacity: 104) { charPtr in
            _ = socketPath.withCString { strlcpy(charPtr, $0, 104) }
        }
    }

    let connected = withUnsafePointer(to: &addr) {
        $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
            connect(sock, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
        }
    }

    guard connected == 0 else {
        throw CLIError("Rack.app is not running — open it first")
    }

    var data = try JSONSerialization.data(withJSONObject: message)
    data.append(UInt8(ascii: "\n"))
    _ = data.withUnsafeBytes { write(sock, $0.baseAddress, data.count) }

    // Read reply
    var reply = Data()
    var buf = [UInt8](repeating: 0, count: 4096)
    while true {
        let n = read(sock, &buf, buf.count)
        guard n > 0 else { break }
        reply.append(contentsOf: buf[..<n])
        if reply.contains(UInt8(ascii: "\n")) { break }
    }

    guard let json = reply.split(separator: UInt8(ascii: "\n")).first else { return nil }
    return try JSONSerialization.jsonObject(with: Data(json)) as? [String: Any]
}

// MARK: - Name inference (mirrors NameInferrer in the app)

func inferName(at directory: URL) -> String {
    func shell(_ cmd: String) -> String? {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/bin/sh")
        p.arguments = ["-c", cmd]
        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = Pipe()
        guard (try? p.run()) != nil else { return nil }
        p.waitUntilExit()
        return String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    func sanitize(_ s: String) -> String {
        s.lowercased().replacingOccurrences(of: " ", with: "-")
    }

    // Base name
    var base = directory.lastPathComponent
    if let remote = shell("git -C '\(directory.path)' remote get-url origin 2>/dev/null"),
       !remote.isEmpty,
       let part = remote.components(separatedBy: "/").last {
        base = part.replacingOccurrences(of: ".git", with: "")
    } else if let pkgData = try? Data(contentsOf: directory.appending(path: "package.json")),
              let pkg = try? JSONSerialization.jsonObject(with: pkgData) as? [String: Any],
              let name = pkg["name"] as? String, !name.isEmpty {
        base = name.components(separatedBy: "/").last ?? name
    }

    // Worktree branch
    if let raw = shell("git -C '\(directory.path)' worktree list --porcelain 2>/dev/null"),
       !raw.isEmpty {
        var worktrees: [(path: String, branch: String?)] = []
        var current: [String: String] = [:]
        for line in (raw + "\n").components(separatedBy: "\n") {
            if line.isEmpty {
                if let path = current["worktree"] {
                    let branch = current["branch"]?.replacingOccurrences(of: "refs/heads/", with: "")
                    worktrees.append((path: path, branch: branch))
                }
                current = [:]
            } else {
                let parts = line.split(separator: " ", maxSplits: 1)
                if parts.count == 2 { current[String(parts[0])] = String(parts[1]) }
                else if parts.count == 1 { current[String(parts[0])] = "" }
            }
        }

        if let main = worktrees.first, main.path != directory.path,
           let match = worktrees.first(where: { directory.path.hasPrefix($0.path) }),
           let branch = match.branch {
            let segment = branch.components(separatedBy: "/").last ?? branch
            return "\(sanitize(segment)).\(sanitize(base))"
        }
    }

    return sanitize(base)
}

// MARK: - Detection (calls PluginRunner logic inline)

struct DetectedCommand {
    let command: String
    let portFlag: String?
}

func detectCommand(at directory: URL) -> DetectedCommand? {
    let fm = FileManager.default
    let files = Set((try? fm.contentsOfDirectory(atPath: directory.path)) ?? [])

    func has(_ f: String) -> Bool { files.contains(f) }
    func content(_ f: String) -> String? { try? String(contentsOf: directory.appending(path: f)) }
    func pm() -> String {
        if has("bun.lockb") { return "bun" }
        if has("pnpm-lock.yaml") { return "pnpm" }
        if has("yarn.lock") { return "yarn" }
        return "npm"
    }

    // Vite — needs --port flag, ignores PORT env var
    if files.contains(where: { $0.hasPrefix("vite.config.") }) {
        return DetectedCommand(command: "\(pm()) exec vite", portFlag: "--port")
    }

    // Astro — uses --port flag, ignores PORT env var
    if files.contains(where: { $0.hasPrefix("astro.config.") }) {
        return DetectedCommand(command: "\(pm()) run dev", portFlag: "--port")
    }

    // Node — Next.js and CRA respect PORT; others may not
    if let pkg = content("package.json"),
       let json = try? JSONSerialization.jsonObject(with: Data(pkg.utf8)) as? [String: Any],
       let scripts = json["scripts"] as? [String: Any] {
        // Next.js uses -p flag
        let deps = (json["dependencies"] as? [String: Any] ?? [:])
            .merging(json["devDependencies"] as? [String: Any] ?? [:]) { a, _ in a }
        let isNext = deps["next"] != nil
        for script in ["dev", "start", "serve"] {
            if scripts[script] != nil {
                return DetectedCommand(command: "\(pm()) run \(script)", portFlag: isNext ? "-p" : nil)
            }
        }
    }

    if has("Package.swift") { return DetectedCommand(command: "swift run", portFlag: nil) }
    if has("Cargo.toml")    { return DetectedCommand(command: "cargo run", portFlag: nil) }
    if has("go.mod")        { return DetectedCommand(command: "go run .", portFlag: nil) }
    if has("manage.py")     { return DetectedCommand(command: "python manage.py runserver", portFlag: nil) }
    if let gf = content("Gemfile"), gf.contains("rails") { return DetectedCommand(command: "rails server", portFlag: "-p") }
    if has("artisan")       { return DetectedCommand(command: "php artisan serve", portFlag: "--port") }
    if let mf = content("Makefile"), mf.contains("\ndev:") || mf.hasPrefix("dev:") {
        return DetectedCommand(command: "make dev", portFlag: nil)
    }

    return nil
}

// MARK: - Commands

func cmdDev() throws {
    let dir = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    guard let detected = detectCommand(at: dir) else {
        print("rack: couldn't detect a dev command in \(dir.lastPathComponent)")
        print("      supported: Node/Vite/Swift/Rust/Go/Django/Rails/Laravel/Make")
        exit(1)
    }

    let name = inferName(at: dir)
    print("rack: detected  → \(detected.command)")
    print("rack: name      → \(name)")
    print("rack: sending to Rack.app...")

    var payload: [String: Any] = [
        "name": name,
        "command": detected.command,
        "workingDirectory": dir.path,
        "environment": [:] as [String: String],
    ]
    if let portFlag = detected.portFlag {
        payload["portFlag"] = portFlag
    }

    let reply = try send([
        "type": "register",
        "payload": payload,
    ])

    if let url = (reply?["payload"] as? [String: String])?["url"] {
        print("")
        print("✓ \(name)")
        print("  \(url)")
    } else if let err = reply?["payload"] as? String, reply?["type"] as? String == "error" {
        print("rack error: \(err)")
        exit(1)
    }
}

func cmdLs() throws {
    let reply = try send(["type": "list"])
    guard let servers = reply?["payload"] as? [[String: Any]], !servers.isEmpty else {
        print("No servers registered. Run 'rack dev' in a project directory.")
        return
    }

    let nameW = servers.map { ($0["name"] as? String ?? "").count }.max() ?? 4
    print(String(repeating: "─", count: nameW + 40))
    for s in servers {
        let name = s["name"] as? String ?? ""
        let url = s["url"] as? String ?? ""
        let running = s["running"] as? Bool ?? false
        let dot = running ? "●" : "○"
        print("\(dot)  \(name.padding(toLength: nameW, withPad: " ", startingAt: 0))  \(url)")
    }
    print(String(repeating: "─", count: nameW + 40))
}

func cmdStart(_ name: String) throws {
    _ = try send(["type": "start", "payload": name])
    print("✓ started \(name)")
}

func cmdStop(_ name: String) throws {
    _ = try send(["type": "stop", "payload": name])
    print("✓ stopped \(name)")
}

func cmdRemove(_ name: String) throws {
    _ = try send(["type": "remove", "payload": name])
    print("✓ removed \(name)")
}

// MARK: - Entry point

struct CLIError: Error, CustomStringConvertible {
    let description: String
    init(_ msg: String) { self.description = msg }
}

let args = CommandLine.arguments.dropFirst()

do {
    switch args.first {
    case "dev":
        try cmdDev()
    case "ls", "list":
        try cmdLs()
    case "start":
        guard let name = args.dropFirst().first else { throw CLIError("Usage: rack start <name>") }
        try cmdStart(name)
    case "stop":
        guard let name = args.dropFirst().first else { throw CLIError("Usage: rack stop <name>") }
        try cmdStop(name)
    case "rm", "remove":
        guard let name = args.dropFirst().first else { throw CLIError("Usage: rack rm <name>") }
        try cmdRemove(name)
    default:
        print("rack — dev environment manager")
        print("")
        print("  rack dev              Register this directory with Rack.app")
        print("  rack ls               List registered servers")
        print("  rack start <name>     Start a server")
        print("  rack stop <name>      Stop a server")
        print("  rack rm <name>        Remove a server")
        print("")
        print("Run 'rack dev' in a project directory. Rack.app must be running.")
    }
} catch let error as CLIError {
    fputs("rack: \(error.description)\n", stderr)
    exit(1)
} catch {
    fputs("rack: \(error.localizedDescription)\n", stderr)
    exit(1)
}
