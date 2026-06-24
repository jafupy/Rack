pub fn is_hooks_host(host: &str) -> bool {
    normalize_host(host).is_some_and(|host| host == "rack.local")
}

fn normalize_host(host: &str) -> Option<String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }

    Some(strip_port(&host).trim_end_matches('.').to_string())
}

fn strip_port(host: &str) -> &str {
    if host.starts_with('[') {
        return host;
    }

    match host.rsplit_once(':') {
        Some((name, port)) if port.parse::<u16>().is_ok() => name,
        _ => host,
    }
}
