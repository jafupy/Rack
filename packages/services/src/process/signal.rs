use std::{thread, time::Duration};

use nix::{
    errno::Errno,
    sys::signal::{killpg, Signal},
    unistd::Pid,
};

use super::ProcessError;

const TERM_GRACE: Duration = Duration::from_millis(300);

pub fn terminate_group(service: &str, pgid: i32) -> Result<(), ProcessError> {
    signal_group(service, pgid, Signal::SIGTERM)?;
    thread::sleep(TERM_GRACE);
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
