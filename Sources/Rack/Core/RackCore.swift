import Foundation
import RackCoreFFI

private func rackCoreEventCallback(json: UnsafePointer<CChar>?, context: UnsafeMutableRawPointer?) {
    guard let json, let context else { return }
    let message = String(cString: json)
    let contextValue = UInt(bitPattern: context)

    Task { @MainActor in
        guard let context = UnsafeMutableRawPointer(bitPattern: contextValue) else { return }
        let core = Unmanaged<RackCore>.fromOpaque(context).takeUnretainedValue()
        core.handleEvent(message)
    }
}

@MainActor
final class RackCore {
    static let shared = RackCore()

    private var eventHandler: ((String) -> Void)?
    private var isStarted = false

    private init() {}

    nonisolated static func commandSync(_ json: String) -> String? {
        guard let response = rack_core_command(json) else { return nil }
        defer { rack_core_free_string(response) }
        return String(cString: response)
    }

    func start(eventHandler: @escaping (String) -> Void) {
        guard !isStarted else { return }
        self.eventHandler = eventHandler

        let context = Unmanaged.passUnretained(self).toOpaque()
        let result = rack_core_start("{}", rackCoreEventCallback, context)

        isStarted = result == 0
    }

    func command(_ json: String) -> String? {
        Self.commandSync(json)
    }

    func stop() {
        guard isStarted else { return }
        rack_core_stop()
        isStarted = false
    }

    fileprivate func handleEvent(_ message: String) {
        eventHandler?(message)
    }
}
