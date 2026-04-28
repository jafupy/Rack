//! rack-bridge — unix socket <-> loopback TCP bridge
//!
//! Usage:
//!   rack-bridge --socket <path> --port <n> -- <command> [args...]
//!
//! What it does:
//!   1. Creates and listens on the unix socket
//!   2. Forks and execs the real dev server with PORT/HOST injected
//!   3. Proxies connections between the unix socket and 127.0.0.1:<port>
//!
//! The dev server never knows about the unix socket.
//! RackProxy never sees a TCP port.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

// MARK: - Args

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
                    command: rest.first().cloned().ok_or("command is required after --")?,
                    command_args: rest.iter().skip(1).cloned().collect(),
                });
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    Err("usage: rack-bridge --socket <path> --port <n> -- <command> [args...]".into())
}

// MARK: - Bridge

/// Bidirectional copy between a UnixStream and TcpStream until either closes.
fn bridge(unix: UnixStream, tcp: TcpStream) {
    let unix_read = unix.try_clone().expect("clone unix stream");
    let tcp_read = tcp.try_clone().expect("clone tcp stream");

    // unix -> tcp
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

    // tcp -> unix
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

/// Connect to 127.0.0.1:port, retrying for up to 30 seconds.
fn connect_to_server(port: u16) -> Result<TcpStream, std::io::Error> {
    let addr = format!("127.0.0.1:{port}");
    let deadline = Instant::now() + Duration::from_secs(30);

    loop {
        match TcpStream::connect(&addr) {
            Ok(stream) => {
                // Disable Nagle — we're proxying, latency matters more than batching
                stream.set_nodelay(true)?;
                return Ok(stream);
            }
            Err(_) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(e) => return Err(e),
        }
    }
}

// MARK: - Main

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("rack-bridge: {e}");
            std::process::exit(1);
        }
    };

    // Ignore SIGPIPE — handle broken pipe via write errors instead
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };

    // Remove stale socket
    let socket_path = args.socket_path.clone();
    let _ = fs::remove_file(&socket_path);

    // Register cleanup on SIGTERM / SIGINT
    ctrlc_or_term(socket_path.clone());

    // Bind the unix socket
    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("rack-bridge: bind {}: {e}", socket_path.display());
            std::process::exit(1);
        }
    };

    // Spawn the dev server as a child process with PORT/HOST injected
    let port = args.port;
    let mut child = Command::new(&args.command);
    child
        .args(&args.command_args)
        .env("PORT", port.to_string())
        .env("HOST", "127.0.0.1");

    let mut child_handle = match child.spawn() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("rack-bridge: failed to spawn '{}': {e}", args.command);
            let _ = fs::remove_file(&socket_path);
            std::process::exit(1);
        }
    };

    // Reap child on exit
    let socket_path_reap = socket_path.clone();
    thread::spawn(move || {
        let _ = child_handle.wait();
        let _ = fs::remove_file(&socket_path_reap);
        std::process::exit(0);
    });

    // Accept connections and bridge them to the TCP server
    for stream in listener.incoming() {
        let unix_stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        thread::spawn(move || {
            match connect_to_server(port) {
                Ok(tcp_stream) => {
                    bridge(unix_stream, tcp_stream);
                }
                Err(e) => {
                    let mut s = unix_stream;
                    let body = format!("rack-bridge: server not ready after 30s: {e}");
                    let response = format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = s.write_all(response.as_bytes());
                }
            }
        });
    }

    let _ = fs::remove_file(&socket_path);
}

// MARK: - Signal handling

fn ctrlc_or_term(socket_path: PathBuf) {
    // Spawn a thread per signal that blocks via sigwait, then cleans up and exits
    for sig in [libc::SIGTERM, libc::SIGINT] {
        let path = socket_path.clone();
        thread::spawn(move || unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, sig);
            libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
            let mut received = 0;
            libc::sigwait(&set, &mut received);
            let _ = fs::remove_file(&path);
            std::process::exit(0);
        });
    }
}


