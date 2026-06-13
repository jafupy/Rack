import AppKit
import SwiftUI

@MainActor
struct FunctionsRuntimeSettingsPage: View {
  @EnvironmentObject private var store: ServerStore
  @AppStorage("functionWorkerLimit") private var functionWorkerLimit = 4

  var body: some View {
    SettingsPage(section: .functions, subtitle: "Configure runtime capacity and inspect local function packages.") {
      HStack {
        Button {
          store.reloadFunctions()
        } label: {
          Label("Reload Functions", systemImage: "arrow.clockwise")
        }
        .buttonStyle(.borderedProminent)

        Button {
          revealFunctionsFolder()
        } label: {
          Label("Reveal Folder", systemImage: "folder")
        }

        Spacer()
      }

      Form {
        Section {
          Stepper(value: $functionWorkerLimit, in: 1...32) {
            LabeledContent("Max concurrent workers") {
              Text("\(functionWorkerLimit)")
                .font(.system(size: 13, weight: .semibold, design: .monospaced))
                .monospacedDigit()
            }
          }
        }
      }
      .formStyle(.grouped)

      if store.functions.isEmpty {
        ContentUnavailableView {
          Label("No Functions Installed", systemImage: "curlybraces.square")
        } description: {
          Text("Install local function packages to inspect their routes and schedules.")
        } actions: {
          Button {
            store.reloadFunctions()
          } label: {
            Label("Reload", systemImage: "arrow.clockwise")
          }
        }
        .frame(maxWidth: .infinity, minHeight: 220)
      } else {
        LazyVStack(spacing: 10) {
          ForEach(store.functions) { function in
            FunctionPackageSettingsPanel(function: function)
          }
        }
      }
    }
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

private struct FunctionPackageSettingsPanel: View {
  let function: ServerStore.FunctionSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .firstTextBaseline, spacing: 10) {
        VStack(alignment: .leading, spacing: 3) {
          HStack(spacing: 8) {
            Text(function.name)
              .font(.system(size: 15, weight: .semibold))
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

        Label(function.errors.isEmpty ? "Ready" : "Errors", systemImage: function.errors.isEmpty ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
          .font(.system(size: 11, weight: .medium))
          .foregroundStyle(function.errors.isEmpty ? .green : .orange)
      }

      if !function.routes.isEmpty {
        FunctionRows(title: "Routes", systemImage: "arrow.triangle.branch") {
          ForEach(function.routes) { route in
            FunctionRow(leading: route.method.uppercased(), main: route.path, trailing: route.function)
          }
        }
      }

      if !function.crons.isEmpty {
        FunctionRows(title: "Cron Schedules", systemImage: "clock") {
          ForEach(function.crons) { cron in
            FunctionRow(leading: "CRON", main: cron.schedule, trailing: cron.function)
          }
        }
      }

      if !function.errors.isEmpty {
        FunctionRows(title: "Errors", systemImage: "exclamationmark.triangle.fill") {
          ForEach(function.errors, id: \.self) { error in
            Text(error)
              .font(.system(size: 11, design: .monospaced))
              .foregroundStyle(.orange)
              .frame(maxWidth: .infinity, alignment: .leading)
              .padding(.horizontal, 10)
              .padding(.vertical, 7)
              .background(.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 7))
          }
        }
      }
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    .overlay {
      RoundedRectangle(cornerRadius: 8)
        .stroke(function.errors.isEmpty ? Color.primary.opacity(0.08) : Color.orange.opacity(0.35))
    }
  }
}

private struct FunctionRows<Content: View>: View {
  let title: String
  let systemImage: String
  @ViewBuilder var content: Content

  var body: some View {
    VStack(alignment: .leading, spacing: 7) {
      Label(title, systemImage: systemImage)
        .font(.system(size: 10, weight: .semibold))
        .foregroundStyle(.secondary)
        .textCase(.uppercase)
      VStack(spacing: 6) {
        content
      }
    }
  }
}

private struct FunctionRow: View {
  let leading: String
  let main: String
  let trailing: String

  var body: some View {
    HStack(spacing: 10) {
      Text(leading)
        .font(.system(size: 10, weight: .bold, design: .monospaced))
        .foregroundStyle(.blue)
        .frame(width: 54, alignment: .leading)
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
