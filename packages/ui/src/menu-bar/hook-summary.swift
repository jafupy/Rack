import Foundation

public struct HookSummary: Identifiable, Equatable {
  public let id: UUID
  public let name: String
  public let routes: [HookRoute]
  public let crons: [HookCron]
  public let errors: [String]

  public init(
    id: UUID = UUID(),
    name: String,
    routes: [HookRoute],
    crons: [HookCron],
    errors: [String]
  ) {
    self.id = id
    self.name = name
    self.routes = routes
    self.crons = crons
    self.errors = errors
  }
}

public struct HookRoute: Identifiable, Equatable {
  public let id: UUID
  public let method: String
  public let path: String

  public init(id: UUID = UUID(), method: String, path: String) {
    self.id = id
    self.method = method
    self.path = path
  }
}

public struct HookCron: Identifiable, Equatable {
  public let id: UUID
  public let schedule: String
  public let hook: String

  public init(id: UUID = UUID(), schedule: String, hook: String) {
    self.id = id
    self.schedule = schedule
    self.hook = hook
  }
}
