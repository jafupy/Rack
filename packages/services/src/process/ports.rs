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
    Ok(stdout
        .lines()
        .filter_map(parse_port)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn parse_port(line: &str) -> Option<u16> {
    let port = line.rsplit_once(':')?.1;
    let port = port.split_whitespace().next()?;
    let port = port.strip_suffix("->").unwrap_or(port);
    port.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsof_listen_port() {
        let line = "node 123 user 20u IPv4 TCP 127.0.0.1:5173 (LISTEN)";

        assert_eq!(parse_port(line), Some(5173));
    }

    #[test]
    fn ignores_lines_without_ports() {
        assert_eq!(parse_port("COMMAND PID USER FD TYPE NAME"), None);
    }

    #[test]
    fn dedupes_and_sorts_ports_from_lsof_output() {
        let output = r#"
node 1 user 20u IPv4 TCP 127.0.0.1:3001 (LISTEN)
node 1 user 21u IPv6 TCP [::1]:3000 (LISTEN)
node 1 user 22u IPv4 TCP 127.0.0.1:3001 (LISTEN)
"#;

        let ports: Vec<_> = output
            .lines()
            .filter_map(parse_port)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        assert_eq!(ports, vec![3000, 3001]);
    }
}
