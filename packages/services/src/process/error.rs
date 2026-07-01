use std::io;

use nix::errno::Errno;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("failed to start service {service}: {source}")]
    StartFailed {
        service: String,
        #[source]
        source: io::Error,
    },
    #[error("missing process handle for running service: {0}")]
    MissingHandle(String),
    #[error("unexpected process handle for stopped service: {0}")]
    UnexpectedHandle(String),
    #[error("process registry desync for service: {0}")]
    RegistryDesync(String),
    #[error("failed to inspect service {service} ports: {source}")]
    PortScanFailed {
        service: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to wait for service {service} shutdown: {source}")]
    WaitFailed {
        service: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to stop service {service} process group {pgid}: {source}")]
    StopFailed {
        service: String,
        pgid: i32,
        #[source]
        source: Errno,
    },
}
