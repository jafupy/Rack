import AppKit
import Darwin
import Foundation
import SwiftUI

@MainActor
final class ServerStore: ObservableObject {
  struct FunctionSummary: Codable, Identifiable, Equatable {
    struct Route: Codable, Identifiable, Equatable {
      var id: String
      var path: String
      var method: String
      var function: String
    }

    struct Cron: Codable, Identifiable, Equatable {
      var id: String
      var schedule: String
      var function: String
    }

    var id: String { name }
    var name: String
    var version: String
    var root: String
    var routes: [Route]
    var crons: [Cron]
    var errors: [String]
  }

  enum AppPaths {
    static let appName = "Rack."
    static let temporaryDirectoryName = "Rack"
    static let commandFilePrefix = "rack"
    static let storageDirectoryName = "rack"
    static let legacyDefaultsBundleID = "dev.jafu.ServerBar"
  }

  @Published var servers: [ServerConfiguration] = []
  @Published var functions: [FunctionSummary] = []
  @Published var selectedServerID: ServerConfiguration.ID?
  @Published var statuses: [ServerConfiguration.ID: ServerStatus] = [:]
  @Published var logs: [ServerConfiguration.ID: String] = [:]

  var logFilePaths: [ServerConfiguration.ID: URL] = [:]
  var logFileHandles: [ServerConfiguration.ID: FileHandle] = [:]
  var terminationSignalSources: [DispatchSourceSignal] = []
  var isHandlingTerminationSignal = false
  let encoder = JSONEncoder()
  let decoder = JSONDecoder()

  init() {
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
    load()

    if selectedServerID == nil {
      selectedServerID = servers.first?.id
    }
    syncIPCContext()

    Task {
      autoStartServers()
    }

    installTerminationSignalHandlers()
  }

  var selectedServer: Binding<ServerConfiguration>? {
    guard let selectedServerID else { return nil }
    return binding(for: selectedServerID)
  }

  var configurationURL: URL {
    let dir = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".config/\(AppPaths.storageDirectoryName)")
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    return dir.appending(path: "config.json")
  }

  func addServer() {
    guard let configuration = addDefaultServerInCore() else {
      NSSound.beep()
      return
    }
    applyCoreConfiguration(configuration)
    selectedServerID = servers.last?.id
  }

  func addServer(_ server: ServerConfiguration) {
    guard let configuration = addServerInCore(server) else {
      NSSound.beep()
      return
    }
    applyCoreConfiguration(configuration)
    selectedServerID = server.id
  }

  func deleteServers(at offsets: IndexSet) {
    let deletedIDs = offsets.map { servers[$0].id }
    for index in offsets {
      let id = servers[index].id
      stopServer(id: id)
    }

    guard let configuration = deleteServersInCore(ids: deletedIDs) else {
      NSSound.beep()
      return
    }
    applyCoreConfiguration(configuration)
  }

  func duplicateSelectedServer() {
    guard let sourceID = selectedServerID else { return }
    let previousIDs = Set(servers.map(\.id))
    guard let configuration = duplicateServerInCore(id: sourceID) else {
      NSSound.beep()
      return
    }
    applyCoreConfiguration(configuration)
    selectedServerID = servers.last(where: { !previousIDs.contains($0.id) })?.id ?? servers.last?.id
  }

  func deleteSelectedServer() {
    guard let selectedServerID, let index = servers.firstIndex(where: { $0.id == selectedServerID })
    else {
      return
    }

    deleteServers(at: IndexSet(integer: index))
  }

  func binding(for id: ServerConfiguration.ID) -> Binding<ServerConfiguration>? {
    guard servers.contains(where: { $0.id == id }) else {
      return nil
    }

    return Binding(
      get: {
        self.servers.first(where: { $0.id == id }) ?? ServerConfiguration(id: id)
      },
      set: { value in
        guard let index = self.servers.firstIndex(where: { $0.id == id }) else {
          return
        }
        self.servers[index] = value
        self.save()
      }
    )
  }

  func status(for id: ServerConfiguration.ID) -> ServerStatus {
    statuses[id] ?? .stopped
  }

  func log(for id: ServerConfiguration.ID) -> String {
    logs[id] ?? ""
  }

  func revealConfigurationFile() {
    NSWorkspace.shared.activateFileViewerSelecting([configurationURL])
  }

  func reloadServers() {
    load()
    if !servers.contains(where: { $0.id == selectedServerID }) {
      selectedServerID = servers.first?.id
    }
    syncIPCContext()
  }

  func applyIPCHostAction(type: String, idString: String) {
    guard let id = UUID(uuidString: idString) else { return }
    switch type {
    case "start":
      if !servers.contains(where: { $0.id == id }) {
        reloadServers()
      }
      startServer(id: id)
    case "stop":
      stopServer(id: id)
    case "remove":
      stopServer(id: id)
      reloadServers()
    default:
      break
    }
  }

  func applyCoreConfiguration(_ configuration: PersistedConfiguration) {
    servers = configuration.servers
    let serverIDs = Set(servers.map(\.id))
    statuses = statuses.filter { serverIDs.contains($0.key) }
    logs = logs.filter { serverIDs.contains($0.key) }
    for server in servers where statuses[server.id] == nil {
      statuses[server.id] = .stopped
    }
    if !servers.contains(where: { $0.id == selectedServerID }) {
      selectedServerID = servers.first?.id
    }
    syncIPCContext()
  }
}
