use rack_core::config::{validate_config, Config, Service, ValidationError, ValidationErrors};

#[test]
fn accepts_valid_config() {
    assert_eq!(validate_config(&valid_config()), Ok(()));
}

#[test]
fn rejects_empty_service_id() {
    let mut config = valid_config();
    config.services[0].id = " ".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::EmptyServiceId { index: 0 }
        ]))
    );
}

#[test]
fn rejects_duplicate_service_ids() {
    let mut config = valid_config();
    config.services.push(Service {
        id: "service-1".to_string(),
        name: "Worker".to_string(),
        host: "worker".to_string(),
        run: "cargo run --bin worker".to_string(),
        working_dir: "~".to_string(),
        auto_start: false,
    });

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::DuplicateServiceId {
                id: "service-1".to_string()
            }
        ]))
    );
}

#[test]
fn rejects_empty_service_name() {
    let mut config = valid_config();
    config.services[0].name = "".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::EmptyServiceName { index: 0 }
        ]))
    );
}

#[test]
fn rejects_empty_host() {
    let mut config = valid_config();
    config.services[0].host = "\t".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![ValidationError::EmptyHost {
            index: 0
        }]))
    );
}

#[test]
fn rejects_invalid_hosts() {
    for host in ["API", "api.localhost", "api_dev", "-api", "api-"] {
        let mut config = valid_config();
        config.services[0].host = host.to_string();

        assert_eq!(
            validate_config(&config),
            Err(ValidationErrors::new(vec![ValidationError::InvalidHost {
                index: 0,
                host: host.to_string()
            }]))
        );
    }
}

#[test]
fn accepts_hyphenated_hosts() {
    let mut config = valid_config();
    config.services[0].host = "jafu-api2".to_string();

    assert_eq!(validate_config(&config), Ok(()));
}

#[test]
fn rejects_duplicate_hosts() {
    let mut config = valid_config();
    config.services.push(Service {
        id: "service-2".to_string(),
        name: "Worker".to_string(),
        host: "api".to_string(),
        run: "cargo run --bin worker".to_string(),
        working_dir: "~".to_string(),
        auto_start: false,
    });

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::DuplicateHost {
                host: "api".to_string()
            }
        ]))
    );
}

#[test]
fn rejects_empty_run() {
    let mut config = valid_config();
    config.services[0].run = "".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![ValidationError::EmptyRun {
            index: 0
        }]))
    );
}

#[test]
fn rejects_empty_working_dir() {
    let mut config = valid_config();
    config.services[0].working_dir = " ".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::EmptyWorkingDir { index: 0 }
        ]))
    );
}

#[test]
fn returns_all_validation_errors() {
    let mut config = valid_config();
    config.services[0].id = "".to_string();
    config.services[0].name = "".to_string();
    config.services[0].host = "".to_string();
    config.services[0].run = "".to_string();
    config.services[0].working_dir = "".to_string();

    assert_eq!(
        validate_config(&config),
        Err(ValidationErrors::new(vec![
            ValidationError::EmptyServiceId { index: 0 },
            ValidationError::EmptyServiceName { index: 0 },
            ValidationError::EmptyHost { index: 0 },
            ValidationError::EmptyRun { index: 0 },
            ValidationError::EmptyWorkingDir { index: 0 },
        ]))
    );
}

fn valid_config() -> Config {
    Config {
        schema_version: 1,
        use_standard_ports: false,
        terminal: "Ghostty".to_string(),
        services: vec![Service {
            id: "service-1".to_string(),
            name: "API".to_string(),
            host: "api".to_string(),
            run: "cargo run".to_string(),
            working_dir: "~".to_string(),
            auto_start: true,
        }],
    }
}
