import SwiftUI

@MainActor
struct EmptyServicesView: View {
  let addService: () -> Void

  var body: some View {
    VStack(spacing: 10) {
      Image(systemName: "server.rack")
        .font(.system(size: 28))
        .foregroundStyle(.quaternary)
      Text("No Services")
        .font(.system(size: 13, weight: .medium))
        .foregroundStyle(.secondary)
      Button("Add a Service", action: addService)
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
    }
    .frame(maxWidth: .infinity)
    .padding(.vertical, 30)
  }
}
