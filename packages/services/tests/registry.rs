use rack_core::config::Service as ServiceConfig;
use rack_services::registry::{Registry, RegistryError, ServiceState};

#[test]
fn registers_services_by_id() {
    let mut registry = Registry::new();
    registry.register(service("api", "api")).unwrap();

    assert_eq!(registry.resolve("api").unwrap().config.name, "API");
}

#[test]
fn rejects_duplicate_ids() {
    let mut registry = Registry::new();
    registry.register(service("api", "api")).unwrap();

    let error = registry.register(service("api", "worker")).unwrap_err();

    assert!(matches!(error, RegistryError::AlreadyRegistered(id) if id == "api"));
}

#[test]
fn rejects_duplicate_hosts() {
    let mut registry = Registry::new();
    registry.register(service("api", "api")).unwrap();

    let error = registry.register(service("worker", "api")).unwrap_err();

    assert!(matches!(error, RegistryError::HostAlreadyRegistered(host) if host == "api"));
}

#[test]
fn transitions_from_stopped_to_starting_to_running() {
    let mut registry = registered();

    registry.mark_starting("api").unwrap();
    registry.mark_spawned("api", 10, 10).unwrap();
    assert_eq!(
        registry.status("api").unwrap(),
        ServiceState::Starting { pid: 10, pgid: 10 }
    );

    registry.mark_running("api", 10, 10, vec![3000]).unwrap();
    assert_eq!(
        registry.status("api").unwrap(),
        ServiceState::Running {
            pid: 10,
            pgid: 10,
            ports: vec![3000]
        }
    );
}

#[test]
fn require_started_accepts_starting_and_running() {
    let mut registry = registered();

    assert!(matches!(
        registry.require_started("api"),
        Err(RegistryError::AlreadyStopped(id)) if id == "api"
    ));

    registry.mark_spawned("api", 10, 10).unwrap();
    assert!(registry.require_started("api").is_ok());

    registry.mark_running("api", 10, 10, vec![3000]).unwrap();
    assert!(registry.require_started("api").is_ok());
}

#[test]
fn updates_ports_only_when_running() {
    let mut registry = registered();

    registry.mark_spawned("api", 10, 10).unwrap();
    registry.update_ports("api", vec![3000]).unwrap();
    assert_eq!(
        registry.status("api").unwrap(),
        ServiceState::Starting { pid: 10, pgid: 10 }
    );

    registry.mark_running("api", 10, 10, vec![3000]).unwrap();
    registry.update_ports("api", vec![3001]).unwrap();
    assert_eq!(
        registry.status("api").unwrap(),
        ServiceState::Running {
            pid: 10,
            pgid: 10,
            ports: vec![3001]
        }
    );
}

fn registered() -> Registry {
    let mut registry = Registry::new();
    registry.register(service("api", "api")).unwrap();
    registry
}

fn service(id: &str, host: &str) -> ServiceConfig {
    ServiceConfig {
        id: id.to_string(),
        name: id.to_uppercase(),
        host: host.to_string(),
        run: "echo hi".to_string(),
        working_dir: "~".to_string(),
        auto_start: false,
    }
}
