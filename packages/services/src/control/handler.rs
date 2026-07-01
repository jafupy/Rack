use rack_proxy::{SharedTargets, TargetTable};

use super::{Command, Request, Response};
use crate::{
    registry::{ServiceState, ServiceView},
    runtime::SharedServiceConfigs,
    snapshot::{snapshot_service, Snapshot},
    supervisor::Supervisor,
};

pub(super) fn handle_request(
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
        Command::Add => add_service(request, supervisor, configs, proxy_port, targets),
        Command::Edit => edit_service(request, supervisor, configs, proxy_port, targets),
        Command::Remove => remove_service(request, supervisor, configs, proxy_port, targets),
        Command::Log => service_log(request, supervisor),
    }
}

fn add_service(
    request: Request,
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) -> Result<Response, String> {
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

fn edit_service(
    request: Request,
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) -> Result<Response, String> {
    let id = request.id.ok_or_else(|| "missing service id".to_string())?;
    let service = request
        .service
        .ok_or_else(|| "missing service config".to_string())?;
    mutate_config(|config| rack_core::config::replace_service(config, &id, service.clone()))?;
    supervisor
        .update(service.clone())
        .map_err(|error| error.to_string())?;
    configs
        .write()
        .map_err(|error| error.to_string())?
        .insert(service.id.clone(), service);
    snapshot_response(supervisor, configs, proxy_port, targets)
}

fn remove_service(
    request: Request,
    supervisor: &Supervisor,
    configs: &SharedServiceConfigs,
    proxy_port: u16,
    targets: &SharedTargets,
) -> Result<Response, String> {
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

fn service_log(request: Request, supervisor: &Supervisor) -> Result<Response, String> {
    let id = request.id.ok_or_else(|| "missing service id".to_string())?;
    let log = supervisor.log(id).map_err(|error| error.to_string())?;
    Ok(Response {
        ok: true,
        snapshot: None,
        log: Some(log),
        error: None,
    })
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

fn service_target(service: &ServiceView) -> Option<rack_proxy::ServiceTarget> {
    let ServiceState::Running { ports, .. } = &service.state else {
        return None;
    };

    Some(rack_proxy::ServiceTarget {
        service_id: service.id.clone(),
        host: service.host.clone(),
        port: *ports.first()?,
    })
}
