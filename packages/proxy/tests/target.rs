use rack_proxy::{ServiceTarget, TargetTable};

#[test]
fn resolves_targets_by_service_host() {
    let table = TargetTable::new([target("api", 3000)]);

    assert_eq!(table.resolve("api"), Some(&target("api", 3000)));
    assert_eq!(table.resolve("web"), None);
}

#[test]
fn formats_loopback_target_urls() {
    assert_eq!(target("api", 5173).loopback_url(), "http://127.0.0.1:5173");
}

fn target(host: &str, port: u16) -> ServiceTarget {
    ServiceTarget {
        service_id: host.to_string(),
        host: host.to_string(),
        port,
    }
}
