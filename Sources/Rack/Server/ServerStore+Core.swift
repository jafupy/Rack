import Foundation

extension ServerStore {
  struct CoreSnapshotReply: Decodable {
    struct Payload: Decodable {
      var servers: [ServerConfiguration]
      var functions: [FunctionSummary]
    }

    var payload: Payload
  }

  struct CoreServersReply: Decodable {
    var payload: PersistedConfiguration
  }

  struct CoreCommand<Payload: Encodable>: Encodable {
    var type: String
    var payload: Payload
  }

  struct CoreIPCContext: Encodable {
    struct Status: Encodable {
      var id: String
      var running: Bool
      var pid: Int32?
    }

    var boundPort: Int
    var standardPortsEnabled: Bool
    var statuses: [Status]
  }

  struct CoreLaunchContext: Encodable {
    var bridgePath: String?
  }

  struct CoreLaunchPlanRequest: Encodable {
    var config: ServerConfiguration
    var context: CoreLaunchContext
  }

  struct CoreServerStartReply: Decodable {
    struct Payload: Decodable {
      var pid: Int32
      var plan: ServerLaunchPlan
    }

    var payload: Payload
  }

  struct CoreServerStopRequest: Encodable {
    var id: String
  }

  struct CoreServerIDsRequest: Encodable {
    var ids: [String]
  }

  struct CoreServerIDRequest: Encodable {
    var id: String
  }

  func addDefaultServerInCore() -> PersistedConfiguration? {
    persistedConfigurationCommand(json: #"{"type":"servers.add","payload":null}"#)
  }

  func addServerInCore(_ server: ServerConfiguration) -> PersistedConfiguration? {
    persistedConfigurationCommand(type: "servers.add", payload: server)
  }

  func deleteServersInCore(ids: [ServerConfiguration.ID]) -> PersistedConfiguration? {
    persistedConfigurationCommand(
      type: "servers.delete",
      payload: CoreServerIDsRequest(ids: ids.map(\.uuidString))
    )
  }

  func duplicateServerInCore(id: ServerConfiguration.ID) -> PersistedConfiguration? {
    persistedConfigurationCommand(
      type: "servers.duplicate",
      payload: CoreServerIDRequest(id: id.uuidString)
    )
  }

  func reloadFunctions() {
    guard let json = RackCore.shared.command(#"{"type":"state.snapshot"}"#),
      let data = json.data(using: .utf8),
      let snapshot = try? decoder.decode(CoreSnapshotReply.self, from: data)
    else {
      functions = []
      return
    }

    functions = snapshot.payload.functions
  }

  func syncIPCContext() {
    let statuses = servers.map { config in
      let status = status(for: config.id)
      let pid: Int32?
      if case .running(let runningPID) = status {
        pid = runningPID
      } else {
        pid = nil
      }
      return CoreIPCContext.Status(
        id: config.id.uuidString,
        running: status.isRunning,
        pid: pid
      )
    }
    let context = CoreIPCContext(
      boundPort: ProxyServer.boundPort,
      standardPortsEnabled: UserDefaults.standard.bool(forKey: "standardPortsEnabled"),
      statuses: statuses
    )
    do {
      let command = CoreCommand(type: "ipc.context", payload: context)
      let data = try encoder.encode(command)
      guard let json = String(data: data, encoding: .utf8) else { return }
      _ = RackCore.shared.command(json)
    } catch {
      return
    }
  }

  func startServerInCore(config: ServerConfiguration, bridgePath: String?) -> CoreServerStartReply
    .Payload?
  {
    do {
      let command = CoreCommand(
        type: "server.start",
        payload: CoreLaunchPlanRequest(
          config: config,
          context: CoreLaunchContext(bridgePath: bridgePath)
        )
      )
      let data = try encoder.encode(command)
      guard let json = String(data: data, encoding: .utf8),
        let response = RackCore.shared.command(json),
        let responseData = response.data(using: .utf8),
        let reply = try? decoder.decode(CoreServerStartReply.self, from: responseData)
      else { return nil }
      return reply.payload
    } catch {
      return nil
    }
  }

  func stopServerInCore(id: ServerConfiguration.ID) {
    do {
      let command = CoreCommand(
        type: "server.stop",
        payload: CoreServerStopRequest(id: id.uuidString)
      )
      let data = try encoder.encode(command)
      guard let json = String(data: data, encoding: .utf8) else { return }
      _ = RackCore.shared.command(json)
    } catch {
      return
    }
  }

  func autoStartServers() {
    for server in servers where server.autoStart {
      startServer(id: server.id)
    }
  }

  nonisolated static func coreBoolCommand(type: String, payload: [String: Any]) -> Bool {
    guard let payload = corePayload(type: type, payload: payload),
      let object = payload as? [String: Any],
      let ready = object["ready"] as? Bool
    else { return false }
    return ready
  }

  nonisolated static func corePayload(type: String, payload: [String: Any]) -> Any? {
    let command: [String: Any] = ["type": type, "payload": payload]
    guard JSONSerialization.isValidJSONObject(command),
      let data = try? JSONSerialization.data(withJSONObject: command),
      let json = String(data: data, encoding: .utf8),
      let response = RackCore.commandSync(json),
      let responseData = response.data(using: .utf8),
      let decoded = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any]
    else { return nil }
    return decoded["payload"]
  }

  private func persistedConfigurationCommand<Payload: Encodable>(
    type: String,
    payload: Payload
  ) -> PersistedConfiguration? {
    do {
      let command = CoreCommand(type: type, payload: payload)
      let data = try encoder.encode(command)
      guard let json = String(data: data, encoding: .utf8) else { return nil }
      return persistedConfigurationCommand(json: json)
    } catch {
      return nil
    }
  }

  private func persistedConfigurationCommand(json: String) -> PersistedConfiguration? {
    guard let response = RackCore.shared.command(json),
      let data = response.data(using: .utf8),
      let reply = try? decoder.decode(CoreServersReply.self, from: data)
    else { return nil }

    return reply.payload
  }
}
