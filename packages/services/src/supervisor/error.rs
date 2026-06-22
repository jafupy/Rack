use thiserror::Error;

use crate::{process::ProcessError, registry::RegistryError};

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error("supervisor thread stopped")]
    Stopped,
}
