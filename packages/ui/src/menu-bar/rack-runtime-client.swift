@MainActor
public protocol RackRuntimeClient: AnyObject {
  func initialize() throws
  func services() throws -> [ServiceConfiguration]
  func startService(id: String) throws
  func stopService(id: String) throws
  func restartService(id: String) throws
  func addService(_ service: ServiceDefinition) throws
  func editService(id: String, service: ServiceDefinition) throws
  func removeService(id: String) throws

  func shutdown()
  func log(for id: String) -> String
  func logFilePath(for id: String) -> String?
  func openInTerminal(id: String)
  func configPath() -> String?
  func terminalName() -> String
  func setTerminalName(_ terminal: String) throws
  func hooks() -> [HookSummary]
  func reloadHooks() throws
  func removeHook(name: String) throws
  func openHookDirectory(name: String)
}
