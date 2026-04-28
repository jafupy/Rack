import Foundation
import WasmKit

// MARK: - Manifest & result types

struct ProjectManifest: Sendable {
    /// All filenames in the project root (non-recursive)
    let files: Set<String>
    /// Pre-read contents of key files
    let contents: [String: String]

    func has(_ file: String) -> Bool { files.contains(file) }
    func content(_ file: String) -> String? { contents[file] }

    static let keyFiles = [
        "package.json", "Cargo.toml", "go.mod", "Package.swift",
        "Gemfile", "pyproject.toml", "requirements.txt",
        "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "Makefile",
        "vite.config.ts", "vite.config.js", "vite.config.mts",
        "manage.py", "artisan", "composer.json",
    ]

    init(at directory: URL) {
        let fm = FileManager.default
        let allFiles = (try? fm.contentsOfDirectory(atPath: directory.path)) ?? []
        files = Set(allFiles)

        var c: [String: String] = [:]
        for name in Self.keyFiles {
            let url = directory.appending(path: name)
            if let text = try? String(contentsOf: url, encoding: .utf8) {
                c[name] = text
            }
        }
        contents = c
    }
}

struct DevCommand: Sendable {
    let command: String
    let env: [String: String]
    /// Suggested portless name override (nil = infer from git/package.json/dir)
    let name: String?
    /// Flag to inject for frameworks that ignore PORT env var. e.g. "--port"
    let portFlag: String?
}

// MARK: - Detector protocol

protocol Detector: Sendable {
    var priority: UInt32 { get }
    func detect(_ manifest: ProjectManifest) -> DevCommand?
}

// MARK: - Built-in detectors

struct NodeDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard let pkg = manifest.content("package.json"),
              let json = try? JSONSerialization.jsonObject(with: Data(pkg.utf8)) as? [String: Any],
              let scripts = json["scripts"] as? [String: Any]
        else { return nil }

        let pm = packageManager(manifest)
        for script in ["dev", "start", "serve"] {
            if scripts[script] != nil {
                return DevCommand(command: "\(pm) run \(script)", env: [:], name: nil, portFlag: nil)
            }
        }
        return nil
    }
}

struct ViteDetector: Detector {
    let priority: UInt32 = 110 // Higher than generic node — check first

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.files.contains(where: { $0.hasPrefix("vite.config.") }) else { return nil }
        let pm = packageManager(manifest)
        return DevCommand(command: "\(pm) exec vite", env: [:], name: nil, portFlag: "--port")
    }
}

struct SwiftDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.has("Package.swift") else { return nil }
        return DevCommand(command: "swift run", env: [:], name: nil, portFlag: nil)
    }
}

struct RustDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.has("Cargo.toml") else { return nil }
        return DevCommand(command: "cargo run", env: [:], name: nil, portFlag: nil)
    }
}

struct GoDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.has("go.mod") else { return nil }
        return DevCommand(command: "go run .", env: [:], name: nil, portFlag: nil)
    }
}

struct DjangoDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.has("manage.py") else { return nil }
        return DevCommand(command: "python manage.py runserver", env: [:], name: nil, portFlag: nil)
    }
}

struct RailsDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard let gemfile = manifest.content("Gemfile"),
              gemfile.contains("rails")
        else { return nil }
        return DevCommand(command: "rails server", env: [:], name: nil, portFlag: "-p")
    }
}

struct LaravelDetector: Detector {
    let priority: UInt32 = 100

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.has("artisan") else { return nil }
        return DevCommand(command: "php artisan serve", env: [:], name: nil, portFlag: "--port")
    }
}

struct MakeDetector: Detector {
    let priority: UInt32 = 50 // Last resort

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard let makefile = manifest.content("Makefile"),
              makefile.contains("\ndev:") || makefile.hasPrefix("dev:")
        else { return nil }
        return DevCommand(command: "make dev", env: [:], name: nil, portFlag: nil)
    }
}

// MARK: - WASM plugin wrapper

/// Wraps a WASM module that implements the rack detector interface.
/// Interface: plugin reads JSON manifest from stdin, writes JSON DevCommand (or "null") to stdout.
final class WasmDetector: Detector, @unchecked Sendable {
    let priority: UInt32
    private let moduleBytes: [UInt8]
    private let engine: Engine

    init(priority: UInt32, moduleBytes: [UInt8]) {
        self.priority = priority
        self.moduleBytes = moduleBytes
        self.engine = Engine()
    }

    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard let manifestJSON = try? JSONSerialization.data(withJSONObject: manifestDict(manifest)),
              let manifestStr = String(data: manifestJSON, encoding: .utf8)
        else { return nil }

        do {
            let module = try parseWasm(bytes: moduleBytes)
            var imports = Imports()

            // Provide WASI-like fd_write for stdout capture
            var output = Data()
            let store = Store(engine: engine)

            // Set up minimal WASI
            let wasi = try WASIBridgeToHost(
                stdin: manifestStr.data(using: .utf8).map { DataReader(data: $0) },
                stdout: DataWriter(output: &output)
            )
            try wasi.link(to: &imports, store: store)

            let instance = try module.instantiate(store: store, imports: imports)
            if let start = instance.exports[function: "_start"] {
                try start(store, [])
            }

            guard let resultStr = String(data: output, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
                  resultStr != "null",
                  !resultStr.isEmpty,
                  let resultData = resultStr.data(using: .utf8),
                  let result = try? JSONDecoder().decode(WasmDevCommandResult.self, from: resultData)
            else { return nil }

            return DevCommand(
                command: result.command,
                env: result.env ?? [:],
                name: result.name,
                portFlag: result.portFlag
            )
        } catch {
            return nil
        }
    }

    private func manifestDict(_ manifest: ProjectManifest) -> [String: Any] {
        [
            "files": Array(manifest.files),
            "contents": manifest.contents,
        ]
    }
}

private struct WasmDevCommandResult: Decodable {
    let command: String
    let env: [String: String]?
    let name: String?
    let portFlag: String?
}

// MARK: - Plugin runner

final class PluginRunner: Sendable {
    static let shared = PluginRunner()

    private let detectors: [any Detector]

    private init() {
        var d: [any Detector] = [
            ViteDetector(),   // Check vite before generic node
            NodeDetector(),
            SwiftDetector(),
            RustDetector(),
            GoDetector(),
            DjangoDetector(),
            RailsDetector(),
            LaravelDetector(),
            MakeDetector(),
        ]

        // Load WASM plugins from ~/.config/rack/plugins/
        let pluginsDir = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".config/rack/plugins")

        if let entries = try? FileManager.default.contentsOfDirectory(
            at: pluginsDir, includingPropertiesForKeys: nil
        ) {
            for entry in entries.filter({ $0.pathExtension == "wasm" }).sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
                guard let bytes = try? Array(Data(contentsOf: entry)) else { continue }
                // Filename convention: "200-my-framework.wasm" -> priority 200
                let priority = entry.deletingPathExtension().lastPathComponent
                    .components(separatedBy: "-").first
                    .flatMap { UInt32($0) } ?? 200
                d.append(WasmDetector(priority: priority, moduleBytes: bytes))
            }
        }

        detectors = d.sorted { $0.priority > $1.priority }
    }

    func detect(in directory: URL) -> DevCommand? {
        let manifest = ProjectManifest(at: directory)
        for detector in detectors {
            if let cmd = detector.detect(manifest) {
                return cmd
            }
        }
        return nil
    }
}

// MARK: - Helpers

private func packageManager(_ manifest: ProjectManifest) -> String {
    if manifest.has("bun.lockb")      { return "bun" }
    if manifest.has("pnpm-lock.yaml") { return "pnpm" }
    if manifest.has("yarn.lock")      { return "yarn" }
    return "npm"
}

// MARK: - Minimal WASI stubs for WasmKit

// WasmKit's WASI bridge types — adapt as needed for the actual WasmKit API version in use.
// These types provide stdin/stdout plumbing for WASM plugin I/O.

private struct DataReader {
    var data: Data
    var offset: Int = 0
    mutating func read(_ count: Int) -> Data {
        let end = min(offset + count, data.count)
        let slice = data[offset..<end]
        offset = end
        return Data(slice)
    }
}

private struct DataWriter {
    var output: UnsafeMutablePointer<Data>
    init(output: inout Data) {
        self.output = withUnsafeMutablePointer(to: &output) { $0 }
    }
    func write(_ data: Data) {
        output.pointee.append(data)
    }
}
