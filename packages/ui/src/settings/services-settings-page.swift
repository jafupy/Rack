import SwiftUI

struct ServicesSettingsPage: View {
  @EnvironmentObject private var model: RackViewModel

  let services: [ServiceConfiguration]

  @State private var editor: ServiceEditorState?
  @State private var serviceToDelete: ServiceConfiguration?

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "Services",
        subtitle: "Add, edit, remove, and inspect Rack services."
      )

      if services.isEmpty {
        PlaceholderCard(
          systemImage: "plus.app",
          title: "No services configured",
          message: "Add a service to route it through Rack."
        )
      } else {
        SettingsCard {
          VStack(spacing: 0) {
            ForEach(Array(services.enumerated()), id: \.element.id) { index, service in
              ServiceSettingsRow(
                service: service,
                onEdit: { editor = .edit(service.definition) },
                onDelete: { serviceToDelete = service }
              )
              if index < services.count - 1 {
                Divider()
              }
            }
          }
        }
      }

      Button {
        editor = .add(.emptyDraft)
      } label: {
        Label("Add Service", systemImage: "plus")
      }
      .buttonStyle(.borderedProminent)
    }
    .sheet(item: $editor) { editor in
      ServiceEditorSheet(
        mode: editor.mode,
        service: editor.service,
        onCancel: { self.editor = nil },
        onSave: save
      )
    }
    .alert("Remove Service?", isPresented: deleteAlertPresented) {
      Button("Remove", role: .destructive) {
        if let serviceToDelete {
          model.removeService(id: serviceToDelete.id)
        }
        serviceToDelete = nil
      }
      Button("Cancel", role: .cancel) {
        serviceToDelete = nil
      }
    } message: {
      Text(deleteMessage)
    }
  }

  private var deleteAlertPresented: Binding<Bool> {
    Binding(
      get: { serviceToDelete != nil },
      set: { if !$0 { serviceToDelete = nil } }
    )
  }

  private var deleteMessage: String {
    guard let service = serviceToDelete else { return "" }
    return "Remove \(service.name.isEmpty ? service.id : service.name) from Rack?"
  }

  private func save(_ service: ServiceDefinition) {
    switch editor?.mode {
    case .add:
      model.addService(service)
    case .edit:
      model.editService(id: service.id, service: service)
    case .none:
      break
    }
    editor = nil
  }
}
