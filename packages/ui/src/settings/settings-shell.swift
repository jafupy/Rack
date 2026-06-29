import SwiftUI

@MainActor
public struct RackSettingsView: View {
  @EnvironmentObject private var model: RackViewModel
  @State private var selectedSection: SettingsSection
  private let generalState: GeneralSettingsState
  private let generalActions: GeneralSettingsActions

  public init(
    initialSection: SettingsSection = .general,
    generalState: GeneralSettingsState = GeneralSettingsState(),
    generalActions: GeneralSettingsActions = GeneralSettingsActions()
  ) {
    _selectedSection = State(initialValue: initialSection)
    self.generalState = generalState
    self.generalActions = generalActions
  }

  public var body: some View {
    NavigationSplitView {
      List(SettingsSection.allCases, selection: $selectedSection) { section in
        Label(section.title, systemImage: section.systemImage)
          .tag(section)
      }
      .navigationSplitViewColumnWidth(min: 180, ideal: 200)
    } detail: {
      detailView
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .navigationTitle(selectedSection.title)
    }
    .frame(minWidth: 760, minHeight: 480)
  }

  @ViewBuilder
  private var detailView: some View {
    switch selectedSection {
    case .general:
      GeneralSettingsPage(
        runningCount: model.services.filter { model.status(for: $0.id).isRunning }.count,
        totalServiceCount: model.services.count,
        hookCount: model.hooks.count,
        state: generalState,
        actions: generalActions
      )
    case .services:
      ServicesSettingsPage(services: model.services)
    case .hooks:
      HooksSettingsPage(hooks: model.hooks)
    }
  }
}
