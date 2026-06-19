import Foundation
@preconcurrency import NIOCore
@preconcurrency import NIOSSL

extension ProxyServer {
  // MARK: - Standard port forwarding (privileged relay, requires administrator)

  /// Installs a LaunchDaemon that listens on standard localhost web ports and relays
  /// raw TCP traffic to Rack's normal proxy ports.
  /// Shows macOS authentication dialog. Returns true on success.
  @discardableResult
  static func setupPortForwarding() -> Bool {
    lastPortForwardingError = nil

    guard let relayPath = bundledPortRelayPath() else {
      lastPortForwardingError = "Could not find bundled rack-port-relay executable."
      return false
    }

    let certPath: String
    do {
      certPath = try ensureLocalTLSCertificate().certificate
    } catch {
      lastPortForwardingError = "Could not create the local TLS certificate: \(error)"
      return false
    }

    let plist = """
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
      <plist version="1.0">
      <dict>
          <key>Label</key>
          <string>com.jafupy.Rack.portfwd</string>
          <key>ProgramArguments</key>
          <array>
              <string>\(portRelayPath)</string>
              <string>--http-target-port</string>
              <string>\(boundPort)</string>
              <string>--https-target-port</string>
              <string>\(boundTLSPort)</string>
          </array>
          <key>RunAtLoad</key>
          <true/>
          <key>KeepAlive</key>
          <true/>
      </dict>
      </plist>
      """

    let tmpPath = "/tmp/com.jafupy.Rack.portfwd.plist"
    guard (try? plist.write(toFile: tmpPath, atomically: true, encoding: .utf8)) != nil else {
      lastPortForwardingError = "Could not write temporary launchd plist."
      return false
    }

    let escapedCertPath = shellEscape(certPath)
    let escapedRelayPath = shellEscape(relayPath)
    let scriptPath = "/tmp/com.jafupy.Rack.portfwd.sh"
    let setupScript = """
      set -e
      launchctl bootout system '\(daemonPath)' 2>/dev/null || true
      /sbin/pfctl -a com.apple/rack -F all 2>/dev/null || true
      if [ -s '\(pfTokenPath)' ]; then
        /sbin/pfctl -X "$(/bin/cat '\(pfTokenPath)')" 2>/dev/null || true
        rm -f '\(pfTokenPath)'
      fi
      mkdir -p '\(privilegedSupportDirectory)'
      cp \(escapedRelayPath) '\(portRelayPath)'
      chmod 755 '\(portRelayPath)'
      cp '\(tmpPath)' '\(daemonPath)'
      launchctl bootstrap system '\(daemonPath)' 2>/dev/null || true
      launchctl kickstart -k system/com.jafupy.Rack.portfwd
      for port in 80 443; do
        listening=0
        for _ in 1 2 3 4 5 6 7 8 9 10; do
          if /usr/bin/nc -z 127.0.0.1 "$port" >/dev/null 2>&1; then
            listening=1
            break
          fi
          sleep 0.1
        done
        if [ "$listening" != 1 ]; then
          launchctl print system/com.jafupy.Rack.portfwd >&2 || true
          echo "Rack standard-port relay did not start listening on port $port." >&2
          exit 1
        fi
      done
      /usr/bin/security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain \(escapedCertPath) 2>/dev/null || true
      tmp_hosts="$(mktemp)"
      /usr/bin/awk '
        $0 == "\(hostsBeginMarker)" { skip = 1; next }
        $0 == "\(hostsEndMarker)" { skip = 0; next }
        skip != 1 { print }
      ' /etc/hosts > "$tmp_hosts"
      cat >> "$tmp_hosts" <<'RACK_HOSTS'
      \(hostsBeginMarker)
      127.0.0.1 rack.local
      ::1 rack.local
      \(hostsEndMarker)
      RACK_HOSTS
      cp "$tmp_hosts" /etc/hosts
      rm -f "$tmp_hosts"
      /usr/bin/dscacheutil -flushcache 2>/dev/null || true
      /usr/bin/killall -HUP mDNSResponder 2>/dev/null || true
      """
    guard (try? setupScript.write(toFile: scriptPath, atomically: true, encoding: .utf8)) != nil
    else {
      lastPortForwardingError = "Could not write temporary standard-port setup script."
      return false
    }

    // Install daemon, trust the local cert, and add rack.local DNS.
    let script = """
      do shell script "/bin/sh \(shellEscape(scriptPath))" with administrator privileges
      """
    var error: NSDictionary?
    NSAppleScript(source: script)?.executeAndReturnError(&error)

    let ok = error == nil
    if let error {
      lastPortForwardingError =
        (error[NSAppleScript.errorMessage] as? String)
        ?? "Standard-port setup failed."
    }
    UserDefaults.standard.set(ok, forKey: "standardPortsEnabled")
    return ok
  }

  /// Removes the standard-port relay LaunchDaemon. Also clears the old pf-based
  /// implementation if the user enabled standard ports in an earlier Rack build.
  static func teardownPortForwarding() {
    lastPortForwardingError = nil

    let scriptPath = "/tmp/com.jafupy.Rack.portfwd-teardown.sh"
    let teardownScript = """
      launchctl bootout system '\(daemonPath)' 2>/dev/null || true
      /sbin/pfctl -a com.apple/rack -F all 2>/dev/null || true
      if [ -s '\(pfTokenPath)' ]; then
        /sbin/pfctl -X "$(/bin/cat '\(pfTokenPath)')" 2>/dev/null || true
        rm -f '\(pfTokenPath)'
      fi
      rm -f '\(daemonPath)'
      rm -f '\(portRelayPath)'
      tmp_hosts="$(mktemp)"
      /usr/bin/awk '
        $0 == "\(hostsBeginMarker)" { skip = 1; next }
        $0 == "\(hostsEndMarker)" { skip = 0; next }
        skip != 1 { print }
      ' /etc/hosts > "$tmp_hosts"
      cp "$tmp_hosts" /etc/hosts
      rm -f "$tmp_hosts"
      /usr/bin/dscacheutil -flushcache 2>/dev/null || true
      /usr/bin/killall -HUP mDNSResponder 2>/dev/null || true
      true
      """
    guard (try? teardownScript.write(toFile: scriptPath, atomically: true, encoding: .utf8)) != nil
    else {
      UserDefaults.standard.set(false, forKey: "standardPortsEnabled")
      return
    }

    let script = """
      do shell script "/bin/sh \(shellEscape(scriptPath))" with administrator privileges
      """
    var error: NSDictionary?
    NSAppleScript(source: script)?.executeAndReturnError(&error)
    UserDefaults.standard.set(false, forKey: "standardPortsEnabled")
  }

  private static func shellEscape(_ value: String) -> String {
    "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
  }

  private static func bundledPortRelayPath() -> String? {
    if let override = ProcessInfo.processInfo.environment["RACK_PORT_RELAY_PATH"],
      FileManager.default.isExecutableFile(atPath: override)
    {
      return override
    }
    if let url = Bundle.main.resourceURL?.appending(path: "rack-port-relay"),
      FileManager.default.isExecutableFile(atPath: url.path)
    {
      return url.path
    }
    let cwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
    for path in [
      ".build/debug/RackPortRelay",
      ".build/arm64-apple-macosx/debug/RackPortRelay",
      ".build/x86_64-apple-macosx/debug/RackPortRelay",
      ".build/release/RackPortRelay",
      ".build/arm64-apple-macosx/release/RackPortRelay",
      ".build/x86_64-apple-macosx/release/RackPortRelay",
    ] {
      let candidate = cwd.appending(path: path).path
      if FileManager.default.isExecutableFile(atPath: candidate) {
        return candidate
      }
    }
    return nil
  }

  static func hasRackLocalHostsEntry() -> Bool {
    guard let hosts = try? String(contentsOfFile: "/etc/hosts", encoding: .utf8) else {
      return false
    }

    return hosts.contains(hostsBeginMarker)
      && hosts.contains("127.0.0.1 rack.local")
      && hosts.contains("::1 rack.local")
      && hosts.contains(hostsEndMarker)
  }

  static func hasCurrentPortForwardingDaemon() -> Bool {
    guard let plist = try? String(contentsOfFile: daemonPath, encoding: .utf8) else {
      return false
    }

    let hasRelay = plist.contains(portRelayPath)
      && plist.contains("--http-target-port")
      && plist.contains("--https-target-port")
    let hasScopedRules = plist.contains("to 127.0.0.1 port 80 -> 127.0.0.1 port")
      && plist.contains("to 127.0.0.1 port 443 -> 127.0.0.1 port")
      && plist.contains("to ::1 port 80 -> ::1 port")
      && plist.contains("to ::1 port 443 -> ::1 port")
    let hasLegacyRules = plist.contains("from any to any port 80 -> 127.0.0.1 port")
      && plist.contains("from any to any port 443 -> 127.0.0.1 port")

    let hasPFForwarding = plist.contains("/sbin/pfctl -E") && (hasScopedRules || hasLegacyRules)
    return hasRelay || hasPFForwarding
  }

  enum TLSCertificateError: Error {
    case creationFailed
  }

  static func makeTLSContext() throws -> NIOSSLContext {
    let paths = try ensureLocalTLSCertificate()
    let certs = try NIOSSLCertificate.fromPEMFile(paths.certificate)
    let key = try NIOSSLPrivateKey(file: paths.privateKey, format: .pem)
    let configuration = TLSConfiguration.makeServerConfiguration(
      certificateChain: certs.map { .certificate($0) },
      privateKey: .privateKey(key)
    )
    return try NIOSSLContext(configuration: configuration)
  }

  private static func ensureLocalTLSCertificate() throws -> (
    certificate: String, privateKey: String
  ) {
    let tlsDir = FileManager.default.homeDirectoryForCurrentUser
      .appending(path: ".config/rack/tls")
    try FileManager.default.createDirectory(at: tlsDir, withIntermediateDirectories: true)

    let certPath = tlsDir.appending(path: "rack-local.pem").path
    let keyPath = tlsDir.appending(path: "rack-local-key.pem").path
    if FileManager.default.fileExists(atPath: certPath),
      FileManager.default.fileExists(atPath: keyPath)
    {
      return (certPath, keyPath)
    }

    let configPath = tlsDir.appending(path: "rack-local-openssl.cnf").path
    let config = """
      [req]
      distinguished_name=req_distinguished_name
      x509_extensions=v3_req
      prompt=no
      [req_distinguished_name]
      CN=rack.local
      [v3_req]
      keyUsage=critical,digitalSignature,keyEncipherment
      extendedKeyUsage=serverAuth
      subjectAltName=@alt_names
      [alt_names]
      DNS.1=localhost
      DNS.2=*.localhost
      DNS.3=rack.local
      """
    try config.write(toFile: configPath, atomically: true, encoding: .utf8)

    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
    process.arguments = [
      "req", "-x509", "-newkey", "rsa:2048", "-sha256", "-days", "825", "-nodes",
      "-keyout", keyPath,
      "-out", certPath,
      "-config", configPath,
    ]
    try process.run()
    process.waitUntilExit()
    guard process.terminationStatus == 0 else {
      throw TLSCertificateError.creationFailed
    }
    return (certPath, keyPath)
  }
}
