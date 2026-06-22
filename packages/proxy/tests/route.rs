use rack_proxy::{route_host, HostRoute, RouteError};

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
