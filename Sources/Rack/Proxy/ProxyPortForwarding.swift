import Foundation
@preconcurrency import NIOCore
@preconcurrency import NIOSSL

extension ProxyServer {
  // MARK: - Standard port forwarding (pfctl, requires administrator)

  /// Installs a LaunchDaemon that redirects standard localhost web ports to Rack's proxy
  /// and applies the rules immediately. Uses the com.apple/rack anchor so it doesn't wipe
  /// macOS system pf rules.
  /// Shows macOS authentication dialog. Returns true on success.
  @discardableResult
  static func setupPortForwarding() -> Bool {
    let certPath: String
    do {
      certPath = try ensureLocalTLSCertificate().certificate
    } catch {
      return false
    }

    let rules = """
      rdr pass on lo0 proto tcp from any to any port 80 -> 127.0.0.1 port \(boundPort)
      rdr pass on lo0 proto tcp from any to any port 443 -> 127.0.0.1 port \(boundTLSPort)
      """
    let pfCommand =
      rules
      .split(separator: "\n")
      .map { "'\($0)'" }
      .joined(separator: " ")

    let plist = """
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
      <plist version="1.0">
      <dict>
          <key>Label</key>
          <string>com.jafupy.Rack.portfwd</string>
          <key>ProgramArguments</key>
          <array>
              <string>/bin/sh</string>
              <string>-c</string>
              <string>/sbin/pfctl -E 2&gt;/dev/null || true; printf '%s\\n' \(pfCommand) | /sbin/pfctl -a com.apple/rack -f - 2&gt;/dev/null || true</string>
          </array>
          <key>RunAtLoad</key>
          <true/>
      </dict>
      </plist>
      """

    let tmpPath = "/tmp/com.jafupy.Rack.portfwd.plist"
    guard (try? plist.write(toFile: tmpPath, atomically: true, encoding: .utf8)) != nil else {
      return false
    }

    let escapedCertPath = shellEscape(certPath)
    let scriptPath = "/tmp/com.jafupy.Rack.portfwd.sh"
    let setupScript = """
      set -e
      cp '\(tmpPath)' '\(daemonPath)'
      launchctl bootstrap system '\(daemonPath)' 2>/dev/null || true
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
      /sbin/pfctl -E 2>/dev/null || true
      printf '%s\\n' \(pfCommand) | /sbin/pfctl -a com.apple/rack -f -
      """
    guard (try? setupScript.write(toFile: scriptPath, atomically: true, encoding: .utf8)) != nil
    else {
      return false
    }

    // Install daemon, trust the local cert, add rack.local DNS, and apply pf immediately.
    let script = """
      do shell script "/bin/sh \(shellEscape(scriptPath))" with administrator privileges
      """
    var error: NSDictionary?
    NSAppleScript(source: script)?.executeAndReturnError(&error)

    let ok = error == nil
    UserDefaults.standard.set(ok, forKey: "standardPortsEnabled")
    return ok
  }

  /// Removes the port forwarding LaunchDaemon and immediately flushes the pf anchor.
  static func teardownPortForwarding() {
    let scriptPath = "/tmp/com.jafupy.Rack.portfwd-teardown.sh"
    let teardownScript = """
      launchctl bootout system '\(daemonPath)' 2>/dev/null || true
      /sbin/pfctl -a com.apple/rack -F all 2>/dev/null || true
      rm -f '\(daemonPath)'
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

    return plist.contains("/sbin/pfctl -E")
      && plist.contains("port 80 -> 127.0.0.1 port")
      && plist.contains("port 443 -> 127.0.0.1 port")
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
