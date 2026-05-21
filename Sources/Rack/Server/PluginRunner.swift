import Foundation

// MARK: - Manifest & result types

struct ProjectManifest: Sendable {
    let files: Set<String>
    let contents: [String: String]

    func has(_ file: String) -> Bool { files.contains(file) }
    func content(_ file: String) -> String? { contents[file] }

    static let keyFiles = [
        "package.json", "Cargo.toml", "go.mod", "Package.swift",
        "Gemfile", "pyproject.toml", "requirements.txt",
        "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "Makefile",
        "vite.config.ts", "vite.config.js", "vite.config.mts",
        "manage.py", "artisan",
    ]

    init(at directory: URL) {
        let fm = FileManager.default
        files = Set((try? fm.contentsOfDirectory(atPath: directory.path)) ?? [])
        var c: [String: String] = [:]
        for name in Self.keyFiles {
            if let text = try? String(contentsOf: directory.appending(path: name), encoding: .utf8) {
                c[name] = text
            }
        }
        contents = c
    }
}

struct DevCommand: Sendable {
    let command: String
    let env: [String: String]
    let name: String?
    /// Flag to inject for frameworks that ignore PORT env var, e.g. "--port"
    let portFlag: String?
}

// MARK: - Detector protocol

protocol Detector: Sendable {
    var priority: UInt32 { get }
    func detect(_ manifest: ProjectManifest) -> DevCommand?
}

// MARK: - Built-in detectors

struct ViteDetector: Detector {
    let priority: UInt32 = 110
    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.files.contains(where: { $0.hasPrefix("vite.config.") }) else { return nil }
        return DevCommand(command: "\(packageManager(manifest)) exec vite", env: [:], name: nil, portFlag: "--port")
    }
}

struct AstroDetector: Detector {
    let priority: UInt32 = 110
    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard manifest.files.contains(where: { $0.hasPrefix("astro.config.") }) else { return nil }
        return DevCommand(command: "\(packageManager(manifest)) run dev", env: [:], name: nil, portFlag: "--port")
    }
}

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
        guard let gemfile = manifest.content("Gemfile"), gemfile.contains("rails") else { return nil }
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
    let priority: UInt32 = 50
    func detect(_ manifest: ProjectManifest) -> DevCommand? {
        guard let makefile = manifest.content("Makefile"),
              makefile.contains("\ndev:") || makefile.hasPrefix("dev:")
        else { return nil }
        return DevCommand(command: "make dev", env: [:], name: nil, portFlag: nil)
    }
}

// MARK: - Plugin runner

final class PluginRunner: Sendable {
    static let shared = PluginRunner()

    private let detectors: [any Detector]

    private init() {
        detectors = [
            ViteDetector(),
            AstroDetector(),
            NodeDetector(),
            SwiftDetector(),
            RustDetector(),
            GoDetector(),
            DjangoDetector(),
            RailsDetector(),
            LaravelDetector(),
            MakeDetector(),
        ].sorted { $0.priority > $1.priority }
    }

    func detect(in directory: URL) -> DevCommand? {
        let manifest = ProjectManifest(at: directory)
        for detector in detectors {
            if let cmd = detector.detect(manifest) { return cmd }
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
