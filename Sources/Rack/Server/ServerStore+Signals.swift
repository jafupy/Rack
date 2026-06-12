import AppKit
import Darwin
import Foundation

extension ServerStore {
  func installTerminationSignalHandlers() {
    let signals = [SIGTERM, SIGINT, SIGHUP]

    for signalNumber in signals {
      signal(signalNumber, SIG_IGN)

      let source = DispatchSource.makeSignalSource(signal: signalNumber, queue: .main)
      source.setEventHandler { [weak self] in
        guard let self, !self.isHandlingTerminationSignal else { return }
        self.isHandlingTerminationSignal = true
        self.stopAllServers()
        NSApp.terminate(nil)
      }
      source.resume()
      terminationSignalSources.append(source)
    }
  }
}
