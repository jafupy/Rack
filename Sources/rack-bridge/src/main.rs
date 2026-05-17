//! rack-bridge — unix socket <-> loopback TCP bridge
//!
//! Usage:
//!   rack-bridge --socket <path> --port <n> -- <command> [args...]
//!
//! Flow:
//!   1. Spawns the dev server with PORT/HOST injected
//!   2. Waits for the requested or newly-created loopback TCP listener
//!   3. Creates the unix socket Rack watches for readiness
//!   4. Bridges each unix connection to the discovered TCP listener

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

struct Args {
    socket_path: PathBuf,
    port: u16,
    command: String,
    command_args: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut socket_path = None;
    let mut port = None;
    let mut i = 0;

    while i < raw.len() {
        match raw[i].as_str() {
            "--socket" => {
                i += 1;
                socket_path = Some(PathBuf::from(raw.get(i).ok_or("--socket needs a value")?));
            }
            "--port" => {
                i += 1;
                port = Some(
                    raw.get(i)
                        .ok_or("--port needs a value")?
                        .parse::<u16>()
                        .map_err(|_| "--port must be a number")?,
                );
            }
            "--" => {
                i += 1;
                let rest = raw.get(i..).unwrap_or(&[]);
                return Ok(Args {
                    socket_path: socket_path.ok_or("--socket is required")?,
                    port: port.ok_or("--port is required")?,
                    command: rest
                        .first()
                        .cloned()
                        .ok_or("command is required after --")?,
                    command_args: rest.iter().skip(1).cloned().collect(),
                });
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    Err("usage: rack-bridge --socket <path> --port <n> -- <command> [args...]".into())
}

fn bridge(unix: UnixStream, tcp: TcpStream) {
    let unix_read = unix.try_clone().expect("clone unix stream");
    let tcp_read = tcp.try_clone().expect("clone tcp stream");

    let a2b = thread::spawn(move || {
        let mut src = unix_read;
        let mut dst = tcp;
        let mut buf = vec![0u8; 65536];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if dst.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = dst.shutdown(std::net::Shutdown::Write);
    });

    let b2a = thread::spawn(move || {
        let mut src = tcp_read;
        let mut dst = unix;
        let mut buf = vec![0u8; 65536];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if dst.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = dst.shutdown(std::net::Shutdown::Both);
    });

    let _ = a2b.join();
    let _ = b2a.join();
}

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

fn loopback_listening_ports() -> HashSet<u16> {
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

fn wait_for_backend_port(
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

static CHILD_PGID: AtomicI32 = AtomicI32::new(0);

fn kill_child() {
    let pgid = CHILD_PGID.load(Ordering::SeqCst);
    if pgid > 0 {
        unsafe { libc::kill(-pgid, libc::SIGTERM) };
        thread::sleep(Duration::from_millis(300));
        unsafe { libc::kill(-pgid, libc::SIGKILL) };
    }
}

fn setup_signals(socket_path: PathBuf) {
    for sig in [libc::SIGTERM, libc::SIGINT] {
        let path = socket_path.clone();
        thread::spawn(move || unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, sig);
            let mut received = 0;
            libc::sigwait(&set, &mut received);
            kill_child();
            let _ = fs::remove_file(&path);
            std::process::exit(0);
        });
    }
}

fn rack_search_paths() -> Vec<PathBuf> {
    let mut search_paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        search_paths.extend([
            home.join(".bun/bin"),
            home.join(".local/bin"),
            home.join(".cargo/bin"),
        ]);
    }
    search_paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/opt/homebrew/sbin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ]);

    search_paths
}

fn rack_child_path() -> Option<String> {
    env::join_paths(rack_search_paths())
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn resolve_command(command: &str) -> String {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command.to_string();
    }

    rack_search_paths()
        .into_iter()
        .map(|dir| dir.join(command))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| command.to_string())
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("rack-bridge: {e}");
            std::process::exit(1);
        }
    };

    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGTERM);
        libc::sigaddset(&mut mask, libc::SIGINT);
        libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut());
    }

    let socket_path = args.socket_path.clone();
    let _ = fs::remove_file(&socket_path);

    let port = args.port;
    let baseline_ports = loopback_listening_ports();
    let command_path = resolve_command(&args.command);
    let mut child = Command::new(&command_path);
    if let Some(path) = rack_child_path() {
        child.env("PATH", path);
    }
    child
        .args(&args.command_args)
        .env("PORT", port.to_string())
        .env("HOST", "127.0.0.1");

    unsafe {
        child.pre_exec(move || {
            libc::setpgid(0, 0);
            let mut mask: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut mask);
            libc::sigprocmask(libc::SIG_SETMASK, &mask, std::ptr::null_mut());
            Ok(())
        });
    }

    let mut child_handle = match child.spawn() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("rack-bridge: failed to spawn '{}': {e}", args.command);
            let _ = fs::remove_file(&socket_path);
            std::process::exit(1);
        }
    };

    let child_pgid = child_handle.id() as i32;
    CHILD_PGID.store(child_pgid, Ordering::SeqCst);
    setup_signals(socket_path.clone());

    let socket_path_reap = socket_path.clone();
    thread::spawn(move || {
        let code = child_handle.wait().ok().and_then(|s| s.code()).unwrap_or(1);
        let _ = fs::remove_file(&socket_path_reap);
        std::process::exit(code);
    });

    let deadline = Instant::now() + Duration::from_secs(60);
    let backend_addr = match wait_for_backend_port(port, baseline_ports, child_pgid, deadline) {
        Some(addr) => addr,
        None => {
            eprintln!("rack-bridge: server did not listen on loopback within 60s");
            kill_child();
            let _ = fs::remove_file(&socket_path);
            std::process::exit(1);
        }
    };

    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("rack-bridge: bind {}: {e}", socket_path.display());
            kill_child();
            std::process::exit(1);
        }
    };

    for stream in listener.incoming() {
        let unix_stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        thread::spawn(move || match TcpStream::connect(backend_addr) {
            Ok(tcp_stream) => {
                let _ = tcp_stream.set_nodelay(true);
                bridge(unix_stream, tcp_stream);
            }
            Err(e) => {
                let mut stream = unix_stream;
                let body = format!("rack-bridge: backend not reachable: {e}");
                let response = format!(
                    "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
    }

    let _ = fs::remove_file(&socket_path);
}
