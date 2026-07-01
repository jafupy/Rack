use std::fmt;

use thiserror::Error;

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

    #[error("service at index {index} has invalid host `{host}`")]
    InvalidHost { index: usize, host: String },

    #[error("host `{host}` is used by multiple services")]
    DuplicateHost { host: String },

    #[error("service at index {index} has an empty run command")]
    EmptyRun { index: usize },

    #[error("service at index {index} has an empty working_dir")]
    EmptyWorkingDir { index: usize },
}
