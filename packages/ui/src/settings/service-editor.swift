import SwiftUI

struct ServiceEditorSheet: View {
  let mode: ServiceEditorMode
  let onCancel: () -> Void
  let onSave: (ServiceDefinition) -> Void

  @State private var draft: ServiceEditorDraft

  init(
    mode: ServiceEditorMode,
    service: ServiceDefinition,
    onCancel: @escaping () -> Void,
    onSave: @escaping (ServiceDefinition) -> Void
  ) {
    self.mode = mode
    self.onCancel = onCancel
    self.onSave = onSave
    _draft = State(initialValue: ServiceEditorDraft(service))
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text(mode.title)
        .font(.title2.weight(.semibold))

      Form {
        TextField("ID", text: $draft.id)
          .disabled(mode == .edit)
        TextField("Name", text: $draft.name)
        TextField("Host", text: $draft.host)
        TextField("Run", text: $draft.run)
        TextField("Working Directory", text: $draft.workingDir)
        Toggle("Auto start", isOn: $draft.autoStart)
      }
      .formStyle(.grouped)

      HStack {
        Spacer()
        Button("Cancel", action: onCancel)
        Button(mode.saveTitle) {
          onSave(draft.service)
        }
        .buttonStyle(.borderedProminent)
        .disabled(!draft.isValid)
      }
    }
    .padding(24)
    .frame(width: 520)
  }
}

enum ServiceEditorMode {
  case add
  case edit

  var title: String {
    switch self {
    case .add: "Add Service"
    case .edit: "Edit Service"
    }
  }

  var saveTitle: String {
    switch self {
    case .add: "Add"
    case .edit: "Save"
    }
  }
}

struct ServiceEditorDraft {
  var id: String
  var name: String
  var host: String
  var run: String
  var workingDir: String
  var autoStart: Bool

  init(_ service: ServiceDefinition) {
    id = service.id
    name = service.name
    host = service.host
    run = service.run
    workingDir = service.workingDir
    autoStart = service.autoStart
  }

  var isValid: Bool {
    !id.trimmed.isEmpty
      && !name.trimmed.isEmpty
      && !host.trimmed.isEmpty
      && !run.trimmed.isEmpty
      && !workingDir.trimmed.isEmpty
  }

  var service: ServiceDefinition {
    ServiceDefinition(
      id: id.trimmed,
      name: name.trimmed,
      host: host.trimmed,
      run: run.trimmed,
      workingDir: workingDir.trimmed,
      autoStart: autoStart
    )
  }
}

extension String {
  fileprivate var trimmed: String {
    trimmingCharacters(in: .whitespacesAndNewlines)
  }
}
