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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_service_localhost_hosts() {
        assert_eq!(
            route_host("jaf.localhost").unwrap(),
            HostRoute::Service {
                host: "jaf".to_string()
            }
        );
    }

    #[test]
    fn routes_service_hosts_case_insensitively_and_strips_ports() {
        assert_eq!(
            route_host("API.localhost:8080").unwrap(),
            HostRoute::Service {
                host: "api".to_string()
            }
        );
    }

    #[test]
    fn routes_rack_local_to_control_surface() {
        assert_eq!(route_host("rack.local").unwrap(), HostRoute::RackLocal);
    }

    #[test]
    fn rejects_empty_hosts() {
        assert_eq!(route_host("  "), Err(RouteError::MissingHost));
    }

    #[test]
    fn rejects_unknown_domains() {
        assert_eq!(
            route_host("example.com"),
            Err(RouteError::UnsupportedHost("example.com".to_string()))
        );
    }

    #[test]
    fn rejects_nested_localhost_hosts() {
        assert_eq!(
            route_host("api.dev.localhost"),
            Err(RouteError::UnsupportedHost("api.dev.localhost".to_string()))
        );
    }
}
