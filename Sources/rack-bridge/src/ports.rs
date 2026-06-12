use std::collections::HashSet;
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

fn wait_for_tcp(port: u16, deadline: Instant) -> Option<SocketAddr> {
    let addrs: [SocketAddr; 2] = [
        format!("127.0.0.1:{port}").parse().unwrap(),
        format!("[::1]:{port}").parse().unwrap(),
    ];
    while Instant::now() < deadline {
        for addr in addrs {
            if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(250)) {
                drop(stream);
                return Some(addr);
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    None
}

pub(crate) fn loopback_listening_ports() -> HashSet<u16> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P", "-F", "n"])
        .output();
    let Ok(output) = output else {
        return HashSet::new();
    };
    parse_lsof_ports(&String::from_utf8_lossy(&output.stdout))
}

fn process_group_pids(pgid: i32) -> Vec<i32> {
    let output = Command::new("/bin/ps")
        .args(["-axo", "pid=,pgid="])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse::<i32>().ok()?;
            let process_group = parts.next()?.parse::<i32>().ok()?;
            (process_group == pgid).then_some(pid)
        })
        .collect()
}

fn loopback_listening_ports_for_pids(pids: &[i32]) -> HashSet<u16> {
    if pids.is_empty() {
        return HashSet::new();
    }

    let pid_list = pids
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let output = Command::new("/usr/sbin/lsof")
        .args([
            "-n",
            "-P",
            "-a",
            "-iTCP",
            "-sTCP:LISTEN",
            "-F",
            "n",
            "-p",
            &pid_list,
        ])
        .output();
    let Ok(output) = output else {
        return HashSet::new();
    };

    parse_lsof_ports(&String::from_utf8_lossy(&output.stdout))
}

fn parse_lsof_ports(text: &str) -> HashSet<u16> {
    text.lines()
        .filter_map(|line| line.strip_prefix('n'))
        .filter(|addr| {
            addr.starts_with("127.0.0.1:")
                || addr.starts_with("*:")
                || addr.starts_with("[::1]:")
                || addr.starts_with("::1:")
                || addr.starts_with("*.")
        })
        .filter_map(|addr| addr.rsplit(':').next())
        .filter_map(|port| port.trim_matches(['[', ']']).parse::<u16>().ok())
        .filter(|port| *port > 1024)
        .collect()
}

pub(crate) fn wait_for_backend_port(
    assigned_port: u16,
    baseline_ports: HashSet<u16>,
    child_pgid: i32,
    deadline: Instant,
) -> Option<SocketAddr> {
    while Instant::now() < deadline {
        if let Some(addr) = wait_for_tcp(assigned_port, Instant::now() + Duration::from_millis(1)) {
            return Some(addr);
        }

        let mut child_candidates: Vec<u16> =
            loopback_listening_ports_for_pids(&process_group_pids(child_pgid))
                .into_iter()
                .filter(|port| *port != assigned_port)
                .collect();
        child_candidates.sort_unstable();
        for port in child_candidates {
            if let Some(addr) = wait_for_tcp(port, Instant::now() + Duration::from_millis(1)) {
                eprintln!("rack-bridge: using child backend port {port}");
                return Some(addr);
            }
        }

        let mut candidates: Vec<u16> = loopback_listening_ports()
            .difference(&baseline_ports)
            .copied()
            .collect();
        candidates.sort_unstable();
        for port in candidates {
            if let Some(addr) = wait_for_tcp(port, Instant::now() + Duration::from_millis(1)) {
                eprintln!("rack-bridge: using discovered backend port {port}");
                return Some(addr);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
    None
}
