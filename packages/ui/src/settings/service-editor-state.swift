import Foundation

struct ServiceEditorState: Identifiable {
  let mode: ServiceEditorMode
  let service: ServiceDefinition

  var id: String {
    switch mode {
    case .add: "add-\(service.id)"
    case .edit: "edit-\(service.id)"
    }
  }

  static func add(_ service: ServiceDefinition) -> ServiceEditorState {
    ServiceEditorState(mode: .add, service: service)
  }

  static func edit(_ service: ServiceDefinition) -> ServiceEditorState {
    ServiceEditorState(mode: .edit, service: service)
  }
}

extension ServiceConfiguration {
  var definition: ServiceDefinition {
    ServiceDefinition(
      id: id,
      name: name,
      host: host,
      run: command,
      workingDir: workingDir,
      autoStart: autoStart
    )
  }
}

extension ServiceDefinition {
  static var emptyDraft: ServiceDefinition {
    ServiceDefinition(
      id: UUID().uuidString,
      name: "",
      host: "",
      run: "",
      workingDir: "",
      autoStart: false
    )
  }
}
