use nix::{
    errno::Errno,
    sys::signal::{killpg, Signal},
    unistd::Pid,
};

use super::ProcessError;

pub fn terminate_group(service: &str, pgid: i32) -> Result<(), ProcessError> {
    signal_group(service, pgid, Signal::SIGTERM)
}

pub fn kill_group(service: &str, pgid: i32) -> Result<(), ProcessError> {
    signal_group(service, pgid, Signal::SIGKILL)
}

fn signal_group(service: &str, pgid: i32, signal: Signal) -> Result<(), ProcessError> {
    match killpg(Pid::from_raw(pgid), signal) {
        Ok(()) | Err(Errno::ESRCH) => Ok(()),
        Err(source) => Err(ProcessError::StopFailed {
            service: service.to_string(),
            pgid,
            source,
        }),
    }
}
