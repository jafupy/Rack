import AppIntents
import RackUI

struct RackServiceEntity: AppEntity, Identifiable {
  static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Rack Service")
  static let defaultQuery = RackServiceEntityQuery()

  let id: String
  let name: String
  let command: String
  let localURL: String

  init(_ service: ServiceConfiguration) {
    id = service.id
    name = service.name.isEmpty ? "Unnamed Service" : service.name
    command = [service.command, service.arguments].filter { !$0.isEmpty }.joined(separator: " ")
    localURL = service.localURL
  }

  var displayRepresentation: DisplayRepresentation {
    DisplayRepresentation(
      title: "\(name)",
      subtitle: command.isEmpty ? "\(localURL)" : "\(command)"
    )
  }
}

struct RackServiceEntityQuery: EntityStringQuery, EnumerableEntityQuery {
  @MainActor
  func entities(for identifiers: [RackServiceEntity.ID]) async throws -> [RackServiceEntity] {
    try identifiers.compactMap { try RackIntentBridge.service(id: $0).map(RackServiceEntity.init) }
  }

  @MainActor
  func entities(matching string: String) async throws -> [RackServiceEntity] {
    let query = string.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    guard !query.isEmpty else { return try await allEntities() }
    return try RackIntentBridge.services()
      .filter { service in
        service.name.lowercased().contains(query)
          || service.command.lowercased().contains(query)
          || service.localURL.lowercased().contains(query)
      }
      .map(RackServiceEntity.init)
  }

  @MainActor
  func suggestedEntities() async throws -> [RackServiceEntity] {
    try await allEntities()
  }

  @MainActor
  func allEntities() async throws -> [RackServiceEntity] {
    try RackIntentBridge.services().map(RackServiceEntity.init)
  }
}
