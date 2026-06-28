mod error;
mod ports;
mod signal;

use std::{
    env,
    io::{self, BufRead, BufReader, Read},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rack_core::{config::Service as ServiceConfig, utils::expand_home};

pub use error::ProcessError;
use ports::listen_ports;
pub use ports::parse_listen_ports;
use signal::terminate_group;

const PORT_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct Process {
    child: Child,
    pgid: i32,
    started_at: Instant,
    output: Receiver<String>,
    _output_threads: Vec<JoinHandle<()>>,
}

impl Process {
    pub fn spawn(id: &str, config: &ServiceConfig) -> Result<Self, ProcessError> {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command = Command::new(shell);
        command
            .arg("-c")
            .arg(&config.run)
            .process_group(0)
            .env("FORCE_COLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .env("TERM", "xterm-256color")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if !config.working_dir.is_empty() {
            command.current_dir(expand_home(&config.working_dir));
        }

        let mut child = command
            .spawn()
            .map_err(|source| ProcessError::StartFailed {
                service: id.to_string(),
                source,
            })?;

        let (output, output_threads) = capture_output(&mut child);

        Ok(Self {
            pgid: child.id() as i32,
            started_at: Instant::now(),
            child,
            output,
            _output_threads: output_threads,
        })
    }

    pub fn pid(&self) -> i32 {
        self.child.id() as i32
    }

    pub fn pgid(&self) -> i32 {
        self.pgid
    }

    pub fn kill(&mut self, id: &str) -> Result<(), ProcessError> {
        terminate_group(id, self.pgid)?;
        let _ = self.child.wait();
        Ok(())
    }

    pub fn has_exited(&mut self) -> io::Result<bool> {
        self.child.try_wait().map(|status| status.is_some())
    }

    pub fn drain_output(&mut self) -> Vec<String> {
        self.output.try_iter().collect()
    }

    pub fn ports(&self, id: &str) -> Result<Vec<u16>, ProcessError> {
        listen_ports(id, self.pgid)
    }

    pub fn readiness_timed_out(&self, timeout: Duration) -> bool {
        self.started_at.elapsed() >= timeout
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

fn capture_output(child: &mut Child) -> (Receiver<String>, Vec<JoinHandle<()>>) {
    let (sender, output) = mpsc::channel();
    let mut threads = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        threads.push(read_output(stdout, sender.clone()));
    }

    if let Some(stderr) = child.stderr.take() {
        threads.push(read_output(stderr, sender));
    }

    (output, threads)
}

fn read_output(stream: impl Read + Send + 'static, sender: mpsc::Sender<String>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let _ = sender.send(line.clone());
                }
            }
        }
    })
}
