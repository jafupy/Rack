import SwiftUI

struct HooksSettingsPage: View {
  let hooks: [HookSummary]

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "Hooks",
        subtitle: "Inspect discovered hooks. Hook management remains read-only in this shell."
      )

      if hooks.isEmpty {
        PlaceholderCard(
          systemImage: "point.3.connected.trianglepath.dotted",
          title: "No hooks discovered",
          message: "Hook settings can be added here when the app exposes management APIs."
        )
      } else {
        SettingsCard {
          VStack(spacing: 0) {
            ForEach(Array(hooks.enumerated()), id: \.element.id) { index, hook in
              HookSettingsRow(hook: hook)
              if index < hooks.count - 1 {
                Divider()
              }
            }
          }
        }
      }
    }
  }
}

private struct HookSettingsRow: View {
  let hook: HookSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack {
        Text(hook.name)
          .font(.headline)
        Spacer()
        if !hook.errors.isEmpty {
          Label(
            "\(hook.errors.count) issue\(hook.errors.count == 1 ? "" : "s")",
            systemImage: "exclamationmark.triangle.fill"
          )
          .font(.caption.weight(.medium))
          .foregroundStyle(.orange)
        }
      }

      if hook.routes.isEmpty && hook.crons.isEmpty {
        Text("No routes or crons reported")
          .font(.caption)
          .foregroundStyle(.secondary)
      } else {
        ForEach(hook.routes) { route in
          Text("\(route.method) \(route.path)")
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.secondary)
        }
        ForEach(hook.crons) { cron in
          Text("\(cron.schedule) -> \(cron.hook)")
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.tertiary)
        }
      }
    }
    .padding(.vertical, 12)
  }
}
