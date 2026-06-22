use thiserror::Error;

const LOCALHOST_SUFFIX: &str = ".localhost";
const RACK_HOST: &str = "rack.local";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRoute {
    Service { host: String },
    RackLocal,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RouteError {
    #[error("missing Host header")]
    MissingHost,

    #[error("unsupported Host header `{0}`")]
    UnsupportedHost(String),
}

pub fn route_host(header: &str) -> Result<HostRoute, RouteError> {
    let Some(host) = normalize_host(header) else {
        return Err(RouteError::MissingHost);
    };

    if host == RACK_HOST {
        return Ok(HostRoute::RackLocal);
    }

    let Some(service_host) = host.strip_suffix(LOCALHOST_SUFFIX) else {
        return Err(RouteError::UnsupportedHost(host.to_string()));
    };

    if service_host.is_empty() || service_host.contains('.') {
        return Err(RouteError::UnsupportedHost(host.to_string()));
    }

    Ok(HostRoute::Service {
        host: service_host.to_string(),
    })
}

fn normalize_host(header: &str) -> Option<String> {
    let host = header.trim().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }

    Some(strip_port(&host).to_string())
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
