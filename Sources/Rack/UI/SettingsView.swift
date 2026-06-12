import AppKit
import SwiftUI

private enum SidebarItem: Hashable {
  case general
  case functions
  case server(UUID)
}

@MainActor
struct SettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @EnvironmentObject private var launchAtLogin: LaunchAtLoginController
  @State private var selection: SidebarItem? = .general

  var body: some View {
    NavigationSplitView {
      sidebar
    } detail: {
      detail
    }
    .onChange(of: selection) { _, newValue in
      if case .server(let id) = newValue {
        store.selectedServerID = id
      } else if newValue != .functions {
        store.selectedServerID = nil
      }
    }
  }

  @ViewBuilder
  private var detail: some View {
    switch selection {
    case .general:
      GeneralSettingsView()
        .environmentObject(store)
        .environmentObject(launchAtLogin)
    case .functions:
      FunctionsSettingsView()
        .environmentObject(store)
    case .server:
      if let selectedServer = store.selectedServer {
        ServerEditorView(server: selectedServer)
          .environmentObject(store)
      } else {
        detailEmptyState
      }
    case nil:
      detailEmptyState
    }
  }

  // MARK: - Sidebar

  private var sidebar: some View {
    List(selection: $selection) {
      Label("Overview", systemImage: "rectangle.grid.2x2")
        .tag(SidebarItem.general)

      Label("Functions", systemImage: "function")
        .tag(SidebarItem.functions)

      Section {
        if store.servers.isEmpty {
          Label("No servers", systemImage: "server.rack")
            .foregroundStyle(.secondary)
        } else {
          ForEach(store.servers) { server in
            ServerListRow(server: server).tag(SidebarItem.server(server.id))
          }
          .onDelete(perform: store.deleteServers)
        }
      } header: {
        HStack {
          Text("Servers")
          Spacer()
          Button {
            store.addServer()
            selection = store.servers.last.map { .server($0.id) }
          } label: {
            Image(systemName: "plus")
          }
          .buttonStyle(.plain)
          .foregroundStyle(.secondary)
          .help("Add Server")
        }
        .padding(.trailing, 8)
      }
    }
    .listStyle(.sidebar)
    .navigationTitle("Rack. Settings")
  }

  // MARK: - Detail Empty State

  private var detailEmptyState: some View {
    ContentUnavailableView {
      Label("No Server Selected", systemImage: "slider.horizontal.3")
    } description: {
      Text("Select a server to configure it, or add a new one.")
    } actions: {
      Button("Add Server") { store.addServer() }
        .buttonStyle(.borderedProminent)
    }
  }
}
