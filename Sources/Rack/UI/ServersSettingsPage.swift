import SwiftUI

@MainActor
struct ServersSettingsPage: View {
  @EnvironmentObject private var store: ServerStore

  var body: some View {
    SettingsPage(section: .servers, subtitle: "Define, run, and inspect local development servers.") {
      Button {
        store.addServer()
      } label: {
        Label("Add Server", systemImage: "plus")
      }
      .buttonStyle(.borderedProminent)

      if store.servers.isEmpty {
        ContentUnavailableView {
          Label("No Servers", systemImage: "server.rack")
        } description: {
          Text("Add a server to route it through Rack.")
        } actions: {
          Button {
            store.addServer()
          } label: {
            Label("Add Server", systemImage: "plus")
          }
          .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, minHeight: 260)
      } else {
        LazyVStack(spacing: 14) {
          ForEach(store.servers) { server in
            if let binding = store.binding(for: server.id) {
              ServerSettingsPanel(server: binding)
                .environmentObject(store)
            }
          }
        }
      }
    }
  }
}
