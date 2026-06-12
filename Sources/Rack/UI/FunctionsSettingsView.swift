import AppKit
import SwiftUI

@MainActor
struct FunctionsSettingsView: View {
  @EnvironmentObject private var store: ServerStore
  @AppStorage("functionWorkerLimit") private var functionWorkerLimit = 4

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 18) {
        SettingsPageHeader(
          title: "Functions",
          subtitle: "Inspect local Rust WASI packages, routes, schedules, and worker capacity.",
          systemImage: "function"
        ) {
          Button {
            revealFunctionsFolder()
          } label: {
            Label("Reveal", systemImage: "folder")
          }

          Button {
            store.reloadFunctions()
          } label: {
            Label("Reload", systemImage: "arrow.clockwise")
          }
        }

        Form {
          Section {
            Stepper(value: $functionWorkerLimit, in: 1...32) {
              LabeledContent("Runtime Workers") {
                Text("\(functionWorkerLimit)")
                  .font(.system(size: 13, weight: .semibold, design: .monospaced))
                  .monospacedDigit()
              }
            }
          } footer: {
            Text("Maximum concurrent rack.local function invocations.")
          }
        }
        .formStyle(.grouped)

        if store.functions.isEmpty {
          FunctionEmptyState(
            revealFunctionsFolder: revealFunctionsFolder, reloadFunctions: store.reloadFunctions)
        } else {
          VStack(alignment: .leading, spacing: 10) {
            Text("Installed Packages")
              .font(.system(size: 11, weight: .semibold))
              .foregroundStyle(.secondary)
              .textCase(.uppercase)

            LazyVStack(spacing: 10) {
              ForEach(store.functions) { function in
                FunctionPackagePanel(function: function)
              }
            }
          }
        }
      }
      .frame(maxWidth: 820)
      .frame(maxWidth: .infinity, alignment: .top)
      .padding(.horizontal, 28)
      .padding(.vertical, 24)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .background(.windowBackground)
    .onAppear {
      store.reloadFunctions()
    }
  }

  private func revealFunctionsFolder() {
    let url = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".rack/functions")
    try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    NSWorkspace.shared.activateFileViewerSelecting([url])
  }
}

struct FunctionPackagePanel: View {
  @AppStorage("standardPortsEnabled") private var standardPortsEnabled = false

  let function: ServerStore.FunctionSummary

  private var hasErrors: Bool {
    !function.errors.isEmpty
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .firstTextBaseline, spacing: 10) {
        VStack(alignment: .leading, spacing: 3) {
          HStack(spacing: 8) {
            Text(function.name)
              .font(.system(size: 15, weight: .semibold))
              .lineLimit(1)
            Text(function.version)
              .font(.system(size: 10, weight: .medium, design: .monospaced))
              .foregroundStyle(.secondary)
              .padding(.horizontal, 6)
              .padding(.vertical, 2)
              .background(.quaternary, in: Capsule())
          }
          Text(function.root)
            .font(.system(size: 10, design: .monospaced))
            .foregroundStyle(.tertiary)
            .lineLimit(1)
            .truncationMode(.middle)
        }

        Spacer()

        Label(
          hasErrors ? "Needs attention" : "Ready",
          systemImage: hasErrors ? "exclamationmark.triangle.fill" : "checkmark.circle.fill"
        )
        .font(.system(size: 11, weight: .medium))
        .foregroundStyle(hasErrors ? .orange : .green)
      }

      if !function.routes.isEmpty {
        FunctionEndpointGroup(title: "Routes", systemImage: "arrow.triangle.branch", tint: .blue) {
          ForEach(function.routes) { route in
            FunctionEndpointRow(
              leading: route.method,
              main: functionRouteURL(route.path),
              trailing: route.function,
              tint: .blue
            )
          }
        }
      }

      if !function.crons.isEmpty {
        FunctionEndpointGroup(title: "Schedules", systemImage: "clock", tint: .green) {
          ForEach(function.crons) { cron in
            FunctionEndpointRow(
              leading: "cron",
              main: cron.schedule,
              trailing: cron.function,
              tint: .green
            )
          }
        }
      }

      if !function.errors.isEmpty {
        VStack(alignment: .leading, spacing: 6) {
          ForEach(function.errors, id: \.self) { error in
            Label(error, systemImage: "exclamationmark.triangle.fill")
              .font(.system(size: 11, weight: .medium))
              .foregroundStyle(.orange)
              .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
        .padding(10)
        .background(.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 8))
      }
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(hasErrors ? Color.orange.opacity(0.35) : Color.primary.opacity(0.08))
    }
  }

  private func functionRouteURL(_ path: String) -> String {
    if standardPortsEnabled {
      return "rack.local\(path)"
    }
    return "localhost:\(ProxyServer.boundPort)\(path)"
  }
}

struct FunctionEndpointGroup<Content: View>: View {
  let title: String
  let systemImage: String
  let tint: Color
  @ViewBuilder var content: Content

  var body: some View {
    VStack(alignment: .leading, spacing: 7) {
      Label(title, systemImage: systemImage)
        .font(.system(size: 10, weight: .semibold))
        .foregroundStyle(tint)
        .textCase(.uppercase)
        .tracking(0.3)
      VStack(spacing: 6) {
        content
      }
    }
  }
}

struct FunctionEndpointRow: View {
  let leading: String
  let main: String
  let trailing: String
  let tint: Color

  var body: some View {
    HStack(spacing: 10) {
      Text(leading.uppercased())
        .font(.system(size: 10, weight: .bold, design: .monospaced))
        .foregroundStyle(tint)
        .frame(width: 48, alignment: .leading)
      Text(main)
        .font(.system(size: 12, weight: .medium, design: .monospaced))
        .lineLimit(1)
        .truncationMode(.middle)
      Spacer(minLength: 8)
      Text(trailing)
        .font(.system(size: 11, design: .monospaced))
        .foregroundStyle(.secondary)
        .lineLimit(1)
    }
    .padding(.horizontal, 10)
    .padding(.vertical, 7)
    .background(.quinary, in: RoundedRectangle(cornerRadius: 7))
  }
}

struct SettingsPageHeader<Actions: View>: View {
  let title: String
  let subtitle: String
  let systemImage: String
  @ViewBuilder var actions: Actions

  init(
    title: String,
    subtitle: String,
    systemImage: String,
    @ViewBuilder actions: () -> Actions = { EmptyView() }
  ) {
    self.title = title
    self.subtitle = subtitle
    self.systemImage = systemImage
    self.actions = actions()
  }

  var body: some View {
    HStack(alignment: .center, spacing: 14) {
      ZStack {
        RoundedRectangle(cornerRadius: 8)
          .fill(.tertiary.opacity(0.35))
        Image(systemName: systemImage)
          .font(.system(size: 22, weight: .semibold))
          .foregroundStyle(.primary)
      }
      .frame(width: 52, height: 52)

      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .font(.system(size: 28, weight: .semibold))
          .lineLimit(1)
        Text(subtitle)
          .font(.system(size: 13))
          .foregroundStyle(.secondary)
          .lineLimit(2)
      }

      Spacer(minLength: 16)

      HStack(spacing: 8) {
        actions
      }
      .controlSize(.regular)
    }
  }
}

struct FunctionEmptyState: View {
  let revealFunctionsFolder: () -> Void
  let reloadFunctions: () -> Void

  var body: some View {
    ContentUnavailableView {
      Label("No Functions Installed", systemImage: "curlybraces.square")
    } description: {
      Text("Create a package with `rack fn init`, then add it to Rack.")
    } actions: {
      HStack {
        Button {
          revealFunctionsFolder()
        } label: {
          Label("Reveal Folder", systemImage: "folder")
        }

        Button {
          reloadFunctions()
        } label: {
          Label("Reload", systemImage: "arrow.clockwise")
        }
      }
    }
    .frame(maxWidth: .infinity, minHeight: 180)
  }
}
