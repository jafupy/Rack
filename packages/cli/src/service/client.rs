use anyhow::{bail, Result};
use rack_core::config;
use rack_services::{
    control::{Client, Command as ControlCommand, Request, Response},
    snapshot::Snapshot,
};

pub fn control_request(
    command: ControlCommand,
    id: Option<String>,
    service: Option<config::Service>,
) -> Result<Response> {
    Client::connect_default()
        .request(Request {
            command,
            id,
            service,
        })
        .map_err(|error| anyhow::anyhow!(error))
}

pub fn response_snapshot(response: Response) -> Result<Snapshot> {
    if !response.ok {
        bail!(response
            .error
            .unwrap_or_else(|| "service command failed".to_string()));
    }

    response
        .snapshot
        .ok_or_else(|| anyhow::anyhow!("service command returned no snapshot"))
}
