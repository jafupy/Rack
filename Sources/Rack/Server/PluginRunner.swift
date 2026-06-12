import Foundation

struct DevCommand: Sendable {
    let command: String
    let env: [String: String]
    let name: String?
    let portFlag: String?
}

final class PluginRunner: Sendable {
    static let shared = PluginRunner()

    private init() {}

    func detect(in directory: URL) -> DevCommand? {
        let command: [String: Any] = [
            "type": "dev.detect",
            "payload": ["directory": directory.path],
        ]
        guard JSONSerialization.isValidJSONObject(command),
              let data = try? JSONSerialization.data(withJSONObject: command),
              let json = String(data: data, encoding: .utf8),
              let responseJSON = RackCore.commandSync(json),
              let responseData = responseJSON.data(using: .utf8),
              let response = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any],
              let payload = response["payload"] as? [String: Any]
        else {
            return nil
        }

        return DevCommand(
            command: payload["command"] as? String ?? "",
            env: payload["env"] as? [String: String] ?? [:],
            name: payload["name"] as? String,
            portFlag: payload["portFlag"] as? String
        )
    }
}
