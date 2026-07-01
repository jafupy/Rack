import SwiftUI

@MainActor
struct HookMenuSection: View {
  let hooks: [HookSummary]
  @Binding var isExpanded: Bool

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Button {
        isExpanded.toggle()
      } label: {
        HStack(spacing: 6) {
          Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
            .font(.system(size: 9, weight: .semibold))
            .frame(width: 10)
          Label("Hooks", systemImage: "point.3.connected.trianglepath.dotted")
        }
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(.secondary)
      }
      .buttonStyle(.plain)

      if isExpanded {
        ForEach(hooks) { hook in
          HookMenuRow(hook: hook)
        }
      }
    }
    .padding(.horizontal, 14)
    .padding(.vertical, 10)
  }
}

@MainActor
private struct HookMenuRow: View {
  let hook: HookSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack {
        Text(hook.name)
          .font(.system(size: 12, weight: .medium))
          .lineLimit(1)
        Spacer()
        if !hook.errors.isEmpty {
          Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.orange)
        }
      }

      ForEach(hook.routes.prefix(2)) { route in
        Text("\(route.method) \(hookRouteURL(route.path))")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.secondary)
          .lineLimit(1)
      }

      ForEach(hook.crons.prefix(2)) { cron in
        Text("\(cron.schedule) -> \(cron.hook)")
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(.tertiary)
          .lineLimit(1)
      }
    }
  }

  private func hookRouteURL(_ path: String) -> String {
    "localhost:\(RackProxy.fallbackPort)\(path)"
  }
}
