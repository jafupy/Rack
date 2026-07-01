use std::collections::HashSet;

use super::Config;

mod error;

pub use error::{ValidationError, ValidationErrors};

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

        if !service.host.trim().is_empty() && !is_valid_host(&service.host) {
            errors.push(ValidationError::InvalidHost {
                index,
                host: service.host.clone(),
            });
        }

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

fn is_valid_host(host: &str) -> bool {
    let bytes = host.as_bytes();
    if bytes.is_empty() || bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }

    bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}
