import SwiftUI

struct GeneralSettingsPage: View {
  let runningCount: Int
  let totalServiceCount: Int
  let hookCount: Int

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "General",
        subtitle: "Rack app preferences will live here as configuration APIs become available."
      )

      SettingsCard {
        VStack(alignment: .leading, spacing: 14) {
          LabeledContent("Services", value: "\(runningCount) running of \(totalServiceCount)")
          LabeledContent("Hooks", value: "\(hookCount) discovered")
        }
      }

      PlaceholderCard(
        systemImage: "switch.2",
        title: "General preferences are read-only for now",
        message:
          "This shell intentionally does not mutate Rust configuration yet. Future controls can be wired here once the config API exists."
      )
    }
  }
}
