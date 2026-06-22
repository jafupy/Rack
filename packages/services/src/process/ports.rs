use std::{collections::BTreeSet, process::Command};

use super::ProcessError;

pub fn listen_ports(service: &str, pgid: i32) -> Result<Vec<u16>, ProcessError> {
    let pgid = pgid.to_string();
    let output = Command::new("lsof")
        .args(["-g", &pgid, "-Pan", "-iTCP", "-sTCP:LISTEN"])
        .output()
        .map_err(|source| ProcessError::PortScanFailed {
            service: service.to_string(),
            source,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_listen_ports(&stdout))
}

pub fn parse_listen_ports(output: &str) -> Vec<u16> {
    output
        .lines()
        .filter_map(parse_port)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parse_port(line: &str) -> Option<u16> {
    let port = line.rsplit_once(':')?.1;
    let port = port.split_whitespace().next()?;
    let port = port.strip_suffix("->").unwrap_or(port);
    port.parse().ok()
}
