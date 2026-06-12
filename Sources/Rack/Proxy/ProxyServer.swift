import Dispatch
import Foundation
@preconcurrency import NIOCore
@preconcurrency import NIOHTTP1
import NIOPosix
@preconcurrency import NIOSSL
@preconcurrency import NIOWebSocket

struct UnsafeSendableBox<Value>: @unchecked Sendable {
  let value: Value
}

struct ProxyHostInfo {
  var routeName: String?
  var isRackLocalHost: Bool
  var isLoopbackCandidate: Bool
}

enum RustProxy {
  static func hostInfo(for host: String?) -> ProxyHostInfo {
    var payload: [String: Any] = [:]
    if let host {
      payload["host"] = host
    }
    guard let payload = commandPayload(type: "proxy.host", payload: payload),
      let info = payload["payload"] as? [String: Any]
    else {
      return ProxyHostInfo(routeName: nil, isRackLocalHost: false, isLoopbackCandidate: false)
    }

    return ProxyHostInfo(
      routeName: info["routeName"] as? String,
      isRackLocalHost: info["isRackLocalHost"] as? Bool ?? false,
      isLoopbackCandidate: info["isLoopbackCandidate"] as? Bool ?? false
    )
  }

  static func rackLocalRequest(method: String, uri: String, headers: HTTPHeaders, body: String)
    -> [String: Any]?
  {
    let payload: [String: Any] = [
      "method": method,
      "uri": uri,
      "headers": headers.map { [$0.name, $0.value] },
      "body": body,
    ]
    guard let response = commandPayload(type: "proxy.rackLocalRequest", payload: payload) else {
      return nil
    }
    return response["payload"] as? [String: Any]
  }

  private static func commandPayload(type: String, payload: [String: Any]) -> [String: Any]? {
    let command: [String: Any] = ["type": type, "payload": payload]
    guard JSONSerialization.isValidJSONObject(command),
      let data = try? JSONSerialization.data(withJSONObject: command),
      let json = String(data: data, encoding: .utf8),
      let responseJSON = RackCore.commandSync(json),
      let responseData = responseJSON.data(using: .utf8)
    else {
      return nil
    }
    return try? JSONSerialization.jsonObject(with: responseData) as? [String: Any]
  }
}

/// HTTP/1.1 reverse proxy that routes *.localhost to unix sockets.
/// Runs inside Rack.app — no external daemon, no Node.
final class ProxyServer: @unchecked Sendable {
  static let defaultPort = 1355
  static let defaultTLSPort = 1443
  static let daemonPath = "/Library/LaunchDaemons/com.jafupy.Rack.portfwd.plist"
  static let hostsBeginMarker = "# Rack.app rack.local begin"
  static let hostsEndMarker = "# Rack.app rack.local end"

  // Set after start() binds. Read by Models.localURL and NameInferrer.
  static nonisolated(unsafe) var boundPort: Int = defaultPort
  static nonisolated(unsafe) var boundTLSPort: Int = defaultTLSPort

  private let group = MultiThreadedEventLoopGroup(numberOfThreads: 2)
  private var channels: [any Channel] = []

  func start(port: Int = ProxyServer.defaultPort) async throws {
    let httpBootstrap = makeBootstrap()

    var lastError: Error?
    for candidate in port...(port + 10) {
      do {
        let bound = try await bindPair(bootstrap: httpBootstrap, port: candidate)
        channels.append(contentsOf: bound)
        ProxyServer.boundPort = candidate
        break
      } catch {
        lastError = error
      }
    }

    guard !channels.isEmpty else {
      throw lastError ?? ProxyError.backendUnavailable
    }

    do {
      let tlsContext = try Self.makeTLSContext()
      let httpsBootstrap = makeBootstrap(tlsContext: tlsContext)
      var tlsLastError: Error?
      for candidate in Self.defaultTLSPort...(Self.defaultTLSPort + 10) {
        do {
          let bound = try await bindPair(bootstrap: httpsBootstrap, port: candidate)
          channels.append(contentsOf: bound)
          ProxyServer.boundTLSPort = candidate
          break
        } catch {
          tlsLastError = error
        }
      }
      if ProxyServer.boundTLSPort == Self.defaultTLSPort
        && !channels.contains(where: { channel in
          guard let address = channel.localAddress else { return false }
          return address.port == Self.defaultTLSPort
        })
      {
        throw tlsLastError ?? TLSCertificateError.creationFailed
      }
    } catch {
      print("RackProxy HTTPS listener disabled: \(error)")
    }

    // Sync the UserDefaults "standard ports" flag with the actual files on disk.
    // Old installs may have the daemon but not the rack.local hosts entry.
    UserDefaults.standard.set(
      Self.hasCurrentPortForwardingDaemon() && Self.hasRackLocalHostsEntry(),
      forKey: "standardPortsEnabled"
    )
  }

  private func makeBootstrap(tlsContext: NIOSSLContext? = nil) -> ServerBootstrap {
    ServerBootstrap(group: group)
      .serverChannelOption(.backlog, value: 256)
      .serverChannelOption(.socketOption(.so_reuseaddr), value: 1)
      .childChannelInitializer { channel in
        let upgrader = NIOWebSocketServerUpgrader(
          shouldUpgrade: { channel, head in
            guard rackRoute(for: head.headers["host"].first) != nil else {
              return channel.eventLoop.makeSucceededFuture(nil)
            }
            var responseHeaders = HTTPHeaders()
            if let webSocketProtocol = head.headers["sec-websocket-protocol"].first {
              responseHeaders.add(name: "sec-websocket-protocol", value: webSocketProtocol)
            }
            return channel.eventLoop.makeSucceededFuture(responseHeaders)
          },
          upgradePipelineHandler: { channel, head in
            WebSocketBackendConnector.connect(frontend: channel, head: head)
          }
        )
        let upgradeConfig: NIOHTTPServerUpgradeConfiguration = (
          upgraders: [upgrader],
          completionHandler: { _ in }
        )
        do {
          if let tlsContext {
            try channel.pipeline.syncOperations.addHandler(NIOSSLServerHandler(context: tlsContext))
          }
          try channel.pipeline.syncOperations.configureHTTPServerPipeline(
            withPipeliningAssistance: true,
            withServerUpgrade: upgradeConfig,
            withErrorHandling: true
          )
          try channel.pipeline.syncOperations.addHandler(HTTPProxyHandler())
          return channel.eventLoop.makeSucceededVoidFuture()
        } catch {
          return channel.eventLoop.makeFailedFuture(error)
        }
      }
      .childChannelOption(.socketOption(.so_reuseaddr), value: 1)
      .childChannelOption(.maxMessagesPerRead, value: 16)
  }

  private func bindPair(bootstrap: ServerBootstrap, port: Int) async throws -> [any Channel] {
    let ipv4 = try await bootstrap.bind(host: "127.0.0.1", port: port).get()
    do {
      let ipv6 = try await bootstrap.bind(host: "::1", port: port).get()
      return [ipv4, ipv6]
    } catch {
      try? await ipv4.close().get()
      throw error
    }
  }

  func stop() async throws {
    for channel in channels {
      try await channel.close()
    }
    channels = []
    try await group.shutdownGracefully()
  }

  deinit {
    try? group.syncShutdownGracefully()
  }
}
