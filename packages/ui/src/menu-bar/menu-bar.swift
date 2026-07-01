import SwiftUI

@MainActor
public struct MenuBarContentView: View {
  @EnvironmentObject private var model: RackViewModel
  @State private var hooksExpanded = false
  var openSettings: ((SettingsSection) -> Void)?

  public init(openSettings: ((SettingsSection) -> Void)? = nil) {
    self.openSettings = openSettings
  }

  private var runningCount: Int {
    model.services.filter { model.status(for: $0.id).isRunning }.count
  }

  public var body: some View {
    VStack(spacing: 0) {
      serviceList
      if !model.hooks.isEmpty {
        Divider()
        hookList
      }
      Divider()
      footer
    }
    .frame(width: 340)
    .fixedSize(horizontal: false, vertical: true)
  }

  @ViewBuilder
  private var serviceList: some View {
    if model.services.isEmpty {
      EmptyServicesView { showSettings(.services) }
    } else if model.services.count > 4 {
      ScrollView { serviceRows }
        .frame(maxHeight: 360)
    } else {
      serviceRows
    }
  }

  private var serviceRows: some View {
    LazyVStack(spacing: 0) {
      ForEach(Array(model.services.enumerated()), id: \.element.id) { index, service in
        ServiceMenuRow(service: service)
        if index < model.services.count - 1 {
          Divider().padding(.leading, 36)
        }
      }
    }
  }

  private var hookList: some View {
    HookMenuSection(hooks: Array(model.hooks.prefix(4)), isExpanded: $hooksExpanded)
  }

  private var footer: some View {
    MenuBarFooter(
      runningCount: runningCount,
      hasServices: !model.services.isEmpty,
      openGeneralSettings: { showSettings(.general) }
    )
  }

  private func showSettings(_ section: SettingsSection) {
    openSettings?(section)
  }
}
