import Foundation

struct InferredProject: Sendable {
    let name: String
    let sanitizedName: String
    let localURL: String
}

enum NameInferrer {
    static func infer(at directory: URL) -> InferredProject {
        let command: [String: Any] = [
            "type": "project.infer",
            "payload": [
                "directory": directory.path,
                "standardPortsEnabled": UserDefaults.standard.bool(forKey: "standardPortsEnabled"),
                "proxyPort": ProxyServer.boundPort,
            ],
        ]
        guard JSONSerialization.isValidJSONObject(command),
              let data = try? JSONSerialization.data(withJSONObject: command),
              let json = String(data: data, encoding: .utf8),
              let responseJSON = RackCore.commandSync(json),
              let responseData = responseJSON.data(using: .utf8),
              let response = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any],
              let payload = response["payload"] as? [String: Any],
              let name = payload["name"] as? String,
              let sanitizedName = payload["sanitizedName"] as? String,
              let localURL = payload["localURL"] as? String
        else {
            return InferredProject(name: directory.lastPathComponent, sanitizedName: "", localURL: "")
        }

        return InferredProject(name: name, sanitizedName: sanitizedName, localURL: localURL)
    }
}
