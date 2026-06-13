import SwiftUI

enum SettingsSection: String, CaseIterable, Hashable, Identifiable {
  case general = "General"
  case servers = "Servers"
  case functions = "Functions"

  var id: Self { self }

  var systemImage: String {
    switch self {
    case .general: return "gearshape"
    case .servers: return "server.rack"
    case .functions: return "function"
    }
  }
}

@MainActor
struct SettingsView: View {
  @State private var selection: SettingsSection? = .general

  var body: some View {
    NavigationSplitView {
      List(SettingsSection.allCases, selection: $selection) { section in
        Label(section.rawValue, systemImage: section.systemImage)
          .tag(section)
      }
      .listStyle(.sidebar)
      .navigationTitle("Rack Settings")
    } detail: {
      switch selection ?? .general {
      case .general:
        GeneralSettingsPage()
      case .servers:
        ServersSettingsPage()
      case .functions:
        FunctionsRuntimeSettingsPage()
      }
    }
  }
}

struct SettingsPage<Content: View>: View {
  let section: SettingsSection
  let subtitle: String
  @ViewBuilder var content: Content

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        HStack(alignment: .center, spacing: 12) {
          Image(systemName: section.systemImage)
            .font(.system(size: 22, weight: .semibold))
            .frame(width: 44, height: 44)
            .background(.tertiary.opacity(0.32), in: RoundedRectangle(cornerRadius: 8))

          VStack(alignment: .leading, spacing: 3) {
            Text(section.rawValue)
              .font(.system(size: 28, weight: .semibold))
            Text(subtitle)
              .font(.system(size: 13))
              .foregroundStyle(.secondary)
              .fixedSize(horizontal: false, vertical: true)
          }
        }

        content
      }
      .frame(maxWidth: 860)
      .frame(maxWidth: .infinity, alignment: .top)
      .padding(.horizontal, 28)
      .padding(.vertical, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
  }
}
