import Foundation
import RackCoreFFI

@MainActor
final class RackCore {
    static let shared = RackCore()

    private var eventHandler: ((String) -> Void)?
    private var isStarted = false

    private init() {}

    func start(eventHandler: @escaping (String) -> Void) {
        guard !isStarted else { return }
        self.eventHandler = eventHandler

        let context = Unmanaged.passUnretained(self).toOpaque()
        let result = rack_core_start("{}", { json, context in
            guard let json, let context else { return }
            let core = Unmanaged<RackCore>.fromOpaque(context).takeUnretainedValue()
            let message = String(cString: json)
            Task { @MainActor in
                core.eventHandler?(message)
            }
        }, context)

        isStarted = result == 0
    }

    func command(_ json: String) -> String? {
        guard let response = rack_core_command(json) else { return nil }
        defer { rack_core_free_string(response) }
        return String(cString: response)
    }

    func stop() {
        guard isStarted else { return }
        rack_core_stop()
        isStarted = false
    }
}
