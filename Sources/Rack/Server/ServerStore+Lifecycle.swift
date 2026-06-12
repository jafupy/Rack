import Foundation

extension ServerStore {
  func startServer(id: ServerConfiguration.ID) {
    guard let config = servers.first(where: { $0.id == id }) else { return }
    guard status(for: id) != .starting, !status(for: id).isRunning else { return }

    statuses[id] = .starting
    logs[id] = ""
    syncIPCContext()

    let tmpDir = URL(fileURLWithPath: NSTemporaryDirectory()).appending(
      path: AppPaths.temporaryDirectoryName)
    try? FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
    let logURL = tmpDir.appending(path: "\(id.uuidString).log")
    FileManager.default.createFile(atPath: logURL.path, contents: nil)
    logFilePaths[id] = logURL
    logFileHandles[id] = try? FileHandle(forWritingTo: logURL)

    guard startServerInCore(config: config, bridgePath: RackBridgeLocator.findRackBridge()) != nil
    else {
      statuses[id] = .failed(message: "Could not launch process")
      syncIPCContext()
      return
    }
  }

  func stopServer(id: ServerConfiguration.ID) {
    stopServerInCore(id: id)
    statuses[id] = .stopped
    try? logFileHandles[id]?.close()
    logFileHandles[id] = nil
    syncIPCContext()
  }

  func appendServerOutput(id: ServerConfiguration.ID, output: String) {
    logs[id, default: ""] += output
    if let handle = logFileHandles[id], let data = output.data(using: .utf8) {
      try? handle.write(contentsOf: data)
    }
    let components = logs[id, default: ""].split(separator: "\n", omittingEmptySubsequences: false)
    if components.count > 400 {
      logs[id] = components.suffix(400).joined(separator: "\n")
    }
  }

  func markServerReady(id: ServerConfiguration.ID, pid: Int32) {
    statuses[id] = .running(pid: pid)
    syncIPCContext()
  }

  func markServerFailed(id: ServerConfiguration.ID, message: String) {
    statuses[id] = .failed(message: message)
    try? logFileHandles[id]?.close()
    logFileHandles[id] = nil
    syncIPCContext()
  }

  func handleServerExit(id: ServerConfiguration.ID, status: Int32, plan: ServerLaunchPlan?) {
    statuses[id] = status == 0 ? .stopped : .failed(message: "Exit \(status)")
    try? logFileHandles[id]?.close()
    logFileHandles[id] = nil
    syncIPCContext()
  }

  func logFilePath(for id: ServerConfiguration.ID) -> URL? {
    logFilePaths[id]
  }

  func restartServer(id: ServerConfiguration.ID) {
    stopServer(id: id)
    startServer(id: id)
  }

  func stopAllServers() {
    for server in servers {
      stopServer(id: server.id)
    }
  }
}
