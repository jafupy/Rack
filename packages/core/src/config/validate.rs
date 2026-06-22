use std::{collections::HashSet, fmt};

use thiserror::Error;

use super::Config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors(Vec<ValidationError>);

impl ValidationErrors {
    pub fn new(errors: Vec<ValidationError>) -> Self {
        Self(errors)
    }

    pub fn as_slice(&self) -> &[ValidationError] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<ValidationError> {
        self.0
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_slice() {
            [] => write!(formatter, "config validation failed"),
            [error] => write!(formatter, "config validation failed: {error}"),
            errors => write!(
                formatter,
                "config validation failed with {} errors",
                errors.len()
            ),
        }
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("service at index {index} has an empty id")]
    EmptyServiceId { index: usize },

    #[error("service id `{id}` is used by multiple services")]
    DuplicateServiceId { id: String },

    #[error("service at index {index} has an empty name")]
    EmptyServiceName { index: usize },

    #[error("service at index {index} has an empty host")]
    EmptyHost { index: usize },

    #[error("host `{host}` is used by multiple services")]
    DuplicateHost { host: String },

    #[error("service at index {index} has an empty run command")]
    EmptyRun { index: usize },

    #[error("service at index {index} has an empty working_dir")]
    EmptyWorkingDir { index: usize },
}

pub fn validate_config(config: &Config) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    let mut service_ids = HashSet::new();
    let mut hosts = HashSet::new();

    for (index, service) in config.services.iter().enumerate() {
        push_if_empty(
            &mut errors,
            &service.id,
            ValidationError::EmptyServiceId { index },
        );

        if !service.id.trim().is_empty() && !service_ids.insert(service.id.as_str()) {
            errors.push(ValidationError::DuplicateServiceId {
                id: service.id.clone(),
            });
        }

        push_if_empty(
            &mut errors,
            &service.name,
            ValidationError::EmptyServiceName { index },
        );
        push_if_empty(
            &mut errors,
            &service.host,
            ValidationError::EmptyHost { index },
        );

        if !service.host.trim().is_empty() && !hosts.insert(service.host.as_str()) {
            errors.push(ValidationError::DuplicateHost {
                host: service.host.clone(),
            });
        }

        push_if_empty(
            &mut errors,
            &service.run,
            ValidationError::EmptyRun { index },
        );
        push_if_empty(
            &mut errors,
            &service.working_dir,
            ValidationError::EmptyWorkingDir { index },
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrors::new(errors))
    }
}

fn push_if_empty(errors: &mut Vec<ValidationError>, value: &str, error: ValidationError) {
    if value.trim().is_empty() {
        errors.push(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Service};

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
}
