import SwiftUI

@MainActor
struct MenuBarFooter: View {
  @EnvironmentObject private var model: RackViewModel
  let runningCount: Int
  let hasServices: Bool
  let openGeneralSettings: () -> Void

  var body: some View {
    HStack {
      Button(action: openGeneralSettings) {
        Image(systemName: "gear")
      }
      .buttonStyle(.plain)
      .font(.system(size: 11))
      .foregroundStyle(.secondary)

      statusText
      Spacer()

      if runningCount > 0 {
        Button("Stop All") { model.stopAll() }
          .buttonStyle(.plain)
          .font(.system(size: 11))
          .foregroundStyle(.secondary)
      }

      Button("Quit") {
        model.stopAll()
        NSApplication.shared.terminate(nil)
      }
      .buttonStyle(.plain)
      .font(.system(size: 11))
      .foregroundStyle(.secondary)
    }
    .padding(.horizontal, 14)
    .padding(.vertical, 8)
  }

  @ViewBuilder
  private var statusText: some View {
    if runningCount > 0 {
      HStack(spacing: 5) {
        Circle().fill(.green).frame(width: 6, height: 6)
        Text("\(runningCount) running")
          .font(.system(size: 11))
          .foregroundStyle(.green)
      }
    } else if hasServices {
      Text("All stopped")
        .font(.system(size: 11))
        .foregroundStyle(.tertiary)
    }
  }
}
