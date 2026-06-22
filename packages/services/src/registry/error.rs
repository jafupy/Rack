use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("already registered service: {0}")]
    AlreadyRegistered(String),
    #[error("already registered service host: {0}")]
    HostAlreadyRegistered(String),
    #[error("unknown service: {0}")]
    UnknownService(String),
    #[error("service already started: {0}")]
    AlreadyStarted(String),
    #[error("service already stopped: {0}")]
    AlreadyStopped(String),
}
