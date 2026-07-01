use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use rack_proxy::SharedTargets;

use super::{handler::handle_request, write_json_line, Request, Response};
use crate::{runtime::SharedServiceConfigs, supervisor::Supervisor};

pub struct ControlServer {
    path: PathBuf,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ControlServer {
    pub fn start(
        supervisor: Supervisor,
        configs: SharedServiceConfigs,
        proxy_port: u16,
        targets: SharedTargets,
    ) -> Result<Self, String> {
        let path = super::socket_path();
        let listener = bind_listener(&path)?;

        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();
        let thread = thread::spawn(move || {
            run(
                listener,
                supervisor,
                configs,
                proxy_port,
                targets,
                thread_running,
            )
        });

        Ok(Self {
            path,
            running,
            thread: Some(thread),
        })
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn bind_listener(path: &Path) -> Result<UnixListener, String> {
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(format!(
                "rack control socket is already active at {}",
                path.display()
            ));
        }
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }

    let listener = UnixListener::bind(path).map_err(|error| error.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    Ok(listener)
}

fn run(
    listener: UnixListener,
    supervisor: Supervisor,
    configs: SharedServiceConfigs,
    proxy_port: u16,
    targets: SharedTargets,
    running: Arc<AtomicBool>,
) {
    while running.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) if running.load(Ordering::Acquire) => {
                handle_client(stream, &supervisor, &configs, proxy_port, &targets)
            }
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => eprintln!("control socket accept failed: {error}"),
        }
    }
}

fn handle_client(
    mut stream: UnixStream,
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) {
    let response = read_request(&stream)
        .and_then(|request| handle_request(request, supervisor, configs, proxy_port, targets))
        .unwrap_or_else(Response::error);

    if let Err(error) = write_json_line(&mut stream, &response) {
        eprintln!("control socket response failed: {error}");
    }
}

fn read_request(stream: &UnixStream) -> Result<Request, String> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&line).map_err(|error| error.to_string())
}
