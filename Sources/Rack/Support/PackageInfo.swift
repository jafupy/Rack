import Foundation

/// Parsed contents of `PackageInfo.json` — the single source of truth for
/// app identity, version, and schema level.
struct PackageInfo: Codable {
    let name: String
    let displayName: String
    let version: String
    let channel: String
    let schemaVersion: String
    let identifier: String
    let author: String
    let license: String
    let repository: Repository

    struct Repository: Codable {
        let type: String
        let url: String
    }

    // MARK: - Singleton

    static let shared: PackageInfo = {
        guard let url = Bundle.module.url(forResource: "PackageInfo", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let info = try? JSONDecoder().decode(PackageInfo.self, from: data) else {
            fatalError("PackageInfo.json missing or malformed — required for version & schema tracking.")
        }
        return info
    }()

    // MARK: - Derived values

    /// e.g. "v0.2::DEV"
    var displayVersion: String {
        "v\(version)::\(channel)"
    }

    /// Clean version without channel suffix, for compact UI.
    var shortVersion: String {
        "v\(version)"
    }

    /// Numeric schema version for config migrations.
    var schemaVersionInt: Int {
        Int(schemaVersion) ?? 1
    }

    /// Short marketing string, e.g. "Rack. v0.2::DEV"
    var fullName: String {
        "\(displayName) \(displayVersion)"
    }
}
