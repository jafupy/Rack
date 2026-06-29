import SwiftUI

struct HooksSettingsPage: View {
  @EnvironmentObject private var model: RackViewModel
  let hooks: [HookSummary]

  var body: some View {
    SettingsPageLayout {
      SettingsSectionHeader(
        title: "Hooks",
        subtitle: "Manage hooks deployed under ~/.rack/hooks."
      )

      HStack {
        Button("Reload Hooks") {
          model.reloadHooks()
        }
        Spacer()
      }

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
              HookSettingsRow(
                hook: hook,
                openDirectory: { model.openHookDirectory(name: hook.name) },
                remove: { model.removeHook(name: hook.name) }
              )
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
  let openDirectory: () -> Void
  let remove: () -> Void

  @State private var confirmingRemoval = false

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
        Button("Open") { openDirectory() }
        Button("Remove", role: .destructive) { confirmingRemoval = true }
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
    .confirmationDialog(
      "Remove \(hook.name)?",
      isPresented: $confirmingRemoval,
      titleVisibility: .visible
    ) {
      Button("Remove Hook", role: .destructive) { remove() }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("This deletes ~/.rack/hooks/\(hook.name).")
    }
  }
}
