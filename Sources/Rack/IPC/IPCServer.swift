import Darwin
import Foundation

// MARK: - Message types

struct IPCRegisterPayload: Codable, Sendable {
    let name: String
    let command: String
    let workingDirectory: String
    let environment: [String: String]
    let portFlag: String?
}

enum IPCMessage: Codable, Sendable {
    case register(IPCRegisterPayload)
    case start(name: String)
    case stop(name: String)
    case remove(name: String)
    case list

    enum CodingKeys: String, CodingKey { case type, payload }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type_ = try c.decode(String.self, forKey: .type)
        switch type_ {
        case "register": self = .register(try c.decode(IPCRegisterPayload.self, forKey: .payload))
        case "start":    self = .start(name: try c.decode(String.self, forKey: .payload))
        case "stop":     self = .stop(name: try c.decode(String.self, forKey: .payload))
        case "remove":   self = .remove(name: try c.decode(String.self, forKey: .payload))
        case "list":     self = .list
        default:         throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "Unknown type \(type_)"))
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .register(let p): try c.encode("register", forKey: .type); try c.encode(p, forKey: .payload)
        case .start(let n):    try c.encode("start", forKey: .type);    try c.encode(n, forKey: .payload)
        case .stop(let n):     try c.encode("stop", forKey: .type);     try c.encode(n, forKey: .payload)
        case .remove(let n):   try c.encode("remove", forKey: .type);   try c.encode(n, forKey: .payload)
        case .list:            try c.encode("list", forKey: .type)
        }
    }
}

enum IPCReply: Encodable, Sendable {
    case ok
    case registered(name: String, url: String)
    case servers([IPCServerStatus])
    case error(String)

    enum CodingKeys: String, CodingKey { case type, payload }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ok:
            try c.encode("ok", forKey: .type)
        case .registered(let name, let url):
            try c.encode("registered", forKey: .type)
            try c.encode(["name": name, "url": url], forKey: .payload)
        case .servers(let list):
            try c.encode("servers", forKey: .type)
            try c.encode(list, forKey: .payload)
        case .error(let msg):
            try c.encode("error", forKey: .type)
            try c.encode(msg, forKey: .payload)
        }
    }
}

struct IPCServerStatus: Codable, Sendable {
    let name: String
    let url: String
    let running: Bool
    let pid: Int32?
}

// MARK: - Server

/// Listens on ~/.config/rack/rack.sock for messages from the `rack` CLI.
@MainActor
final class IPCServer {
    private let socketPath: String
    weak var store: ServerStore?

    init() {
        socketPath = FileManager.default.homeDirectoryForCurrentUser
            .appending(path: ".config/rack/rack.sock").path
    }

    func start() {
        // Remove stale socket
        try? FileManager.default.removeItem(atPath: socketPath)
        startPOSIX()
    }

    private func startPOSIX() {
        let path = socketPath
        let store = self.store

        Task.detached {
            let sock = socket(AF_UNIX, SOCK_STREAM, 0)
            guard sock >= 0 else { return }

            var addr = sockaddr_un()
            addr.sun_family = sa_family_t(AF_UNIX)
            withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
                ptr.withMemoryRebound(to: CChar.self, capacity: 104) { charPtr in
                    _ = path.withCString { strlcpy(charPtr, $0, 104) }
                }
            }

            let bindResult = withUnsafePointer(to: &addr) {
                $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    bind(sock, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }
            guard bindResult == 0 else { return }
            guard listen(sock, 16) == 0 else { return }

            while true {
                let client = accept(sock, nil, nil)
                guard client >= 0 else { continue }
                Task.detached {
                    await Self.handleClient(client, store: store)
                }
            }
        }
    }

    private static func handleClient(_ fd: Int32, store: ServerStore?) async {
        defer { close(fd) }

        var data = Data()
        var buf = [UInt8](repeating: 0, count: 4096)

        // Read until newline
        while true {
            let n = read(fd, &buf, buf.count)
            guard n > 0 else { break }
            data.append(contentsOf: buf[..<n])
            if data.contains(UInt8(ascii: "\n")) { break }
        }

        guard
            let json = data.split(separator: UInt8(ascii: "\n")).first,
            let msg = try? JSONDecoder().decode(IPCMessage.self, from: Data(json))
        else {
            let reply = try? JSONEncoder().encode(IPCReply.error("invalid message"))
            _ = reply.map { write(fd, Array($0), $0.count) }
            return
        }

        let reply = await MainActor.run {
            Self.handle(msg, store: store)
        }

        if var replyData = try? JSONEncoder().encode(reply) {
            replyData.append(UInt8(ascii: "\n"))
            replyData.withUnsafeBytes { _ = write(fd, $0.baseAddress, replyData.count) }
        }
    }

    @MainActor
    private static func handle(_ msg: IPCMessage, store: ServerStore?) -> IPCReply {
        guard let store else { return .error("Rack.app store not ready") }

        switch msg {
        case .register(let payload):
            var config = ServerConfiguration()
            config.name = payload.name
            config.command = payload.command
            config.workingDirectory = payload.workingDirectory
            config.environment = payload.environment.map {
                ServerConfiguration.EnvironmentVariable(key: $0.key, value: $0.value)
            }
            config.portFlag = payload.portFlag
            config.autoStart = true
            store.addServer(config)
            store.startServer(id: config.id)
            let url = config.localURL
            return .registered(name: payload.name, url: url)

        case .start(let name):
            guard let id = store.servers.first(where: { $0.name == name })?.id else {
                return .error("no server named '\(name)'")
            }
            store.startServer(id: id)
            return .ok

        case .stop(let name):
            guard let id = store.servers.first(where: { $0.name == name })?.id else {
                return .error("no server named '\(name)'")
            }
            store.stopServer(id: id)
            return .ok

        case .remove(let name):
            guard let index = store.servers.firstIndex(where: { $0.name == name }) else {
                return .error("no server named '\(name)'")
            }
            store.deleteServers(at: IndexSet(integer: index))
            return .ok

        case .list:
            let statuses = store.servers.map { config in
                IPCServerStatus(
                    name: config.name,
                    url: config.localURL,
                    running: store.status(for: config.id).isRunning,
                    pid: {
                        if case .running(let pid) = store.status(for: config.id) { return pid }
                        return nil
                    }()
                )
            }
            return .servers(statuses)
        }
    }
}
