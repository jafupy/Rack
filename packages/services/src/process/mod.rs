mod error;
mod ports;
mod signal;

use std::{
    env, io,
    os::unix::process::CommandExt,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use rack_core::{config::Service as ServiceConfig, utils::expand_home};

pub use error::ProcessError;
use ports::listen_ports;
use signal::terminate_group;

const PORT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct Process {
    child: Child,
    pgid: i32,
}

impl Process {
    pub fn spawn(id: &str, config: &ServiceConfig) -> Result<Self, ProcessError> {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command = Command::new(shell);
        command.arg("-c").arg(&config.run).process_group(0);

        if !config.working_dir.is_empty() {
            command.current_dir(expand_home(&config.working_dir));
        }

        let child = command
            .spawn()
            .map_err(|source| ProcessError::StartFailed {
                service: id.to_string(),
                source,
            })?;

        Ok(Self {
            pgid: child.id() as i32,
            child,
        })
    }

    pub fn pid(&self) -> i32 {
        self.child.id() as i32
    }

    pub fn pgid(&self) -> i32 {
        self.pgid
    }

    pub fn kill(mut self, id: &str) -> Result<(), ProcessError> {
        terminate_group(id, self.pgid)?;
        let _ = self.child.wait();
        Ok(())
    }

    pub fn has_exited(&mut self) -> io::Result<bool> {
        self.child.try_wait().map(|status| status.is_some())
    }

    pub fn ports(&self, id: &str) -> Result<Vec<u16>, ProcessError> {
        listen_ports(id, self.pgid)
    }

    pub fn wait_for_ports(&self, id: &str, timeout: Duration) -> Result<Vec<u16>, ProcessError> {
        let deadline = Instant::now() + timeout;

        loop {
            let ports = self.ports(id)?;
            if !ports.is_empty() || Instant::now() >= deadline {
                return Ok(ports);
            }

            thread::sleep(PORT_POLL_INTERVAL);
        }
    }
}
