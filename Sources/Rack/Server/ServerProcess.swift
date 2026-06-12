import Foundation

struct ServerLaunchPlan: Codable {
    var subdomain: String
    var socketPath: String
    var port: Int
    var useBridge: Bool
    var executable: String
    var arguments: [String]
    var workingDirectory: String
    var environment: [String: String]
    var launchDescription: String
}

enum RackBridgeLocator {
    /// Returns the path to the rack-bridge binary, or nil if not found.
    static func findRackBridge() -> String? {
        if let override = ProcessInfo.processInfo.environment["RACK_BRIDGE_PATH"],
           FileManager.default.isExecutableFile(atPath: override) {
            return override
        }
        if let url = Bundle.main.resourceURL?.appending(path: "rack-bridge"),
           FileManager.default.isExecutableFile(atPath: url.path) {
            return url.path
        }
        let devPath = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appending(path: ".build/rust/release/rack-bridge").path
        if FileManager.default.isExecutableFile(atPath: devPath) {
            return devPath
        }
        return nil
    }
}
