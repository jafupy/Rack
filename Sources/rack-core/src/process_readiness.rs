use serde_json::Value;
use std::collections::HashSet;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, TcpStream};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::time::Duration;

pub(crate) fn readiness_command(command_type: &str, payload: &Value) -> Option<Value> {
    let response = match command_type {
        "server.probeUnixSocket" => probe_unix_socket_command(payload),
        "server.probePort" => probe_port_command(payload),
        "server.loopbackListeningPorts" => Ok(serde_json::json!({
            "type": "server.loopbackListeningPorts",
            "payload": loopback_listening_ports().into_iter().collect::<Vec<_>>(),
        })),
        _ => return None,
    };
    Some(response.unwrap_or_else(|message| {
        serde_json::json!({
            "type": "error",
            "message": message,
        })
    }))
}

pub(crate) fn allocate_port() -> u16 {
    let used = loopback_listening_ports();
    (4000..=4999)
        .find(|port| !used.contains(port) && !probe_port(*port))
        .unwrap_or(4000)
}

pub(crate) fn loopback_listening_ports() -> HashSet<u16> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P", "-F", "n"])
        .output();
    let Ok(output) = output else {
        return HashSet::new();
    };
    parse_lsof_ports(&String::from_utf8_lossy(&output.stdout))
}

pub(crate) fn probe_port(port: u16) -> bool {
    let v4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let v6 = SocketAddrV6::new(Ipv6Addr::LOCALHOST, port, 0, 0);
    TcpStream::connect_timeout(&v4.into(), Duration::from_millis(50)).is_ok()
        || TcpStream::connect_timeout(&v6.into(), Duration::from_millis(50)).is_ok()
}

pub(crate) fn probe_unix_socket(path: &str) -> bool {
    UnixStream::connect(path).is_ok()
}

fn probe_unix_socket_command(payload: &Value) -> Result<Value, String> {
    let path = payload
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing socket path".to_string())?;
    Ok(serde_json::json!({
        "type": "server.probeUnixSocket",
        "payload": {
            "ready": probe_unix_socket(path),
        }
    }))
}

fn probe_port_command(payload: &Value) -> Result<Value, String> {
    let port = payload
        .get("port")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing port".to_string())? as u16;
    Ok(serde_json::json!({
        "type": "server.probePort",
        "payload": {
            "ready": probe_port(port),
        }
    }))
}

fn parse_lsof_ports(text: &str) -> HashSet<u16> {
    text.lines()
        .filter_map(|line| line.strip_prefix('n'))
        .filter(|addr| {
            addr.starts_with("127.0.0.1:")
                || addr.starts_with("*:")
                || addr.starts_with("[::1]:")
                || addr.starts_with("::1:")
                || addr.starts_with("*.")
        })
        .filter_map(|addr| addr.rsplit(':').next())
        .filter_map(|port| port.trim_matches(['[', ']']).parse::<u16>().ok())
        .filter(|port| *port > 1024)
        .collect()
}
