import Foundation
import ServiceManagement

@MainActor
final class LaunchAtLoginController: ObservableObject {
    @Published private(set) var isEnabled = false
    @Published private(set) var errorMessage: String?

    init() {
        refresh()
    }

    func refresh() {
        errorMessage = nil
        isEnabled = Self.isStatusEnabled(SMAppService.mainApp.status)
    }

    func setEnabled(_ enabled: Bool) {
        errorMessage = nil

        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }

            isEnabled = Self.isStatusEnabled(SMAppService.mainApp.status)

            if enabled && !isEnabled {
                errorMessage = Self.message(for: SMAppService.mainApp.status)
            }
        } catch {
            isEnabled = Self.isStatusEnabled(SMAppService.mainApp.status)
            errorMessage = Self.message(for: SMAppService.mainApp.status) ?? error.localizedDescription
        }
    }

    private static func isStatusEnabled(_ status: SMAppService.Status) -> Bool {
        switch status {
        case .enabled, .requiresApproval:
            return true
        case .notFound, .notRegistered:
            return false
        @unknown default:
            return false
        }
    }

    private static func message(for status: SMAppService.Status) -> String? {
        switch status {
        case .requiresApproval:
            return "macOS requires approval for launch at login in System Settings."
        case .notFound:
            return "Launch at login is only available from the built app bundle."
        case .enabled, .notRegistered:
            return nil
        @unknown default:
            return "Rack. could not update its login item state."
        }
    }
}
