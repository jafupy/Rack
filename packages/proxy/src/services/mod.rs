mod destination;
mod routes;

pub use destination::{Destination, ServiceTarget};
pub use routes::{ServiceRoutes, SharedTargets, TargetTable};

const LOCALHOST_SUFFIX: &str = ".localhost";

pub fn origin_from_host(host: &str) -> Option<String> {
    let host = normalize_host(host)?;
    let origin = host.strip_suffix(LOCALHOST_SUFFIX)?;
    (!origin.is_empty() && !origin.contains('.')).then(|| origin.to_string())
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
