use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use rack_proxy::{ServiceTarget, SharedTargets, TargetTable};

use super::{write_json_line, Command, Request, Response};
use crate::{
    registry::{ServiceState, ServiceView},
    runtime::SharedServiceConfigs,
    snapshot::{snapshot_service, Snapshot},
    supervisor::Supervisor,
};

pub struct ControlServer {
    path: std::path::PathBuf,
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
        self.running.store(false, Ordering::Relaxed);
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
    while running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => handle_client(stream, &supervisor, &configs, proxy_port, &targets),
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

fn handle_request(
    request: Request,
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) -> Result<Response, String> {
    match request.command {
        Command::List => snapshot_response(supervisor, configs, proxy_port, targets),
        Command::Start => run_service_command(request.id, supervisor, |supervisor, id| {
            supervisor.start_service(id)
        })
        .and_then(|_| snapshot_response(supervisor, configs, proxy_port, targets)),
        Command::Stop => run_service_command(request.id, supervisor, |supervisor, id| {
            supervisor.stop_service(id)
        })
        .and_then(|_| snapshot_response(supervisor, configs, proxy_port, targets)),
        Command::Restart => run_service_command(request.id, supervisor, |supervisor, id| {
            supervisor.restart_service(id)
        })
        .and_then(|_| snapshot_response(supervisor, configs, proxy_port, targets)),
        Command::Add => {
            let service = request
                .service
                .ok_or_else(|| "missing service config".to_string())?;
            mutate_config(|config| rack_core::config::add_service(config, service.clone()))?;
            supervisor
                .register(service.clone())
                .map_err(|error| error.to_string())?;
            configs
                .write()
                .map_err(|error| error.to_string())?
                .insert(service.id.clone(), service);
            snapshot_response(supervisor, configs, proxy_port, targets)
        }
        Command::Edit => {
            let id = request.id.ok_or_else(|| "missing service id".to_string())?;
            let service = request
                .service
                .ok_or_else(|| "missing service config".to_string())?;
            mutate_config(|config| {
                rack_core::config::replace_service(config, &id, service.clone())
            })?;
            supervisor
                .update(service.clone())
                .map_err(|error| error.to_string())?;
            configs
                .write()
                .map_err(|error| error.to_string())?
                .insert(service.id.clone(), service);
            snapshot_response(supervisor, configs, proxy_port, targets)
        }
        Command::Remove => {
            let id = request.id.ok_or_else(|| "missing service id".to_string())?;
            mutate_config(|config| rack_core::config::remove_service(config, &id).map(|_| ()))?;
            supervisor
                .unregister(&id)
                .map_err(|error| error.to_string())?;
            configs
                .write()
                .map_err(|error| error.to_string())?
                .remove(&id);
            snapshot_response(supervisor, configs, proxy_port, targets)
        }
        Command::Log => {
            let id = request.id.ok_or_else(|| "missing service id".to_string())?;
            let log = supervisor.log(id).map_err(|error| error.to_string())?;
            Ok(Response {
                ok: true,
                snapshot: None,
                log: Some(log),
                error: None,
            })
        }
    }
}

fn mutate_config(
    mutate: impl FnOnce(&mut rack_core::config::Config) -> Result<(), rack_core::config::WriteError>,
) -> Result<(), String> {
    let mut config = rack_core::config::load().map_err(|error| error.to_string())?;
    mutate(&mut config).map_err(|error| error.to_string())?;
    rack_core::config::save(&config).map_err(|error| error.to_string())?;
    Ok(())
}

fn run_service_command(
    id: Option<String>,
    supervisor: &Supervisor,
    command: impl FnOnce(&Supervisor, String) -> Result<(), crate::supervisor::SupervisorError>,
) -> Result<(), String> {
    let id = id.ok_or_else(|| "missing service id".to_string())?;
    command(supervisor, id).map_err(|error| error.to_string())
}

fn snapshot_response(
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) -> Result<Response, String> {
    let views = supervisor.list().map_err(|error| error.to_string())?;
    refresh_targets(targets, &views);
    let configs = configs.read().map_err(|error| error.to_string())?;
    Ok(Response {
        ok: true,
        snapshot: Some(Snapshot {
            proxy_port: Some(proxy_port),
            services: views
                .into_iter()
                .map(|view| snapshot_service(view, &configs))
                .collect(),
        }),
        log: None,
        error: None,
    })
}

fn refresh_targets(targets: &SharedTargets, services: &[ServiceView]) {
    targets.update(TargetTable::new(services.iter().filter_map(service_target)));
}

fn service_target(service: &ServiceView) -> Option<ServiceTarget> {
    let ServiceState::Running { ports, .. } = &service.state else {
        return None;
    };

    Some(ServiceTarget {
        service_id: service.id.clone(),
        host: service.host.clone(),
        port: *ports.first()?,
    })
}
