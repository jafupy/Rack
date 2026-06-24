use rack_proxy::{is_hooks_host, origin_from_host};

#[test]
fn extracts_service_origins_from_localhost_hosts() {
    assert_eq!(origin_from_host("jaf.localhost"), Some("jaf".to_string()));
}

#[test]
fn extracts_origins_case_insensitively_and_strips_ports() {
    assert_eq!(
        origin_from_host("API.localhost:8080"),
        Some("api".to_string())
    );
}

#[test]
fn identifies_hooks_host() {
    assert!(is_hooks_host("rack.local"));
}

#[test]
fn rejects_empty_hosts() {
    assert_eq!(origin_from_host("  "), None);
}

#[test]
fn rejects_unknown_domains() {
    assert_eq!(origin_from_host("example.com"), None);
}

#[test]
fn rejects_nested_localhost_hosts() {
    assert_eq!(origin_from_host("api.dev.localhost"), None);
}
