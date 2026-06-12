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

mod args;
mod ports;
mod process;
mod tunnel;

use std::fs;
use std::os::unix::net::UnixListener;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args = match args::parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("rack-bridge: {error}");
            std::process::exit(1);
        }
    };

    process::block_termination_signals();

    let socket_path = args.socket_path.clone();
    let _ = fs::remove_file(&socket_path);

    let baseline_ports = ports::loopback_listening_ports();
    let (child, child_pgid) = match process::spawn_child(&args) {
        Ok(child) => child,
        Err(error) => {
            eprintln!("rack-bridge: {error}");
            let _ = fs::remove_file(&socket_path);
            std::process::exit(1);
        }
    };

    process::setup_signals(socket_path.clone());
    process::reap_child(child, socket_path.clone());

    let deadline = Instant::now() + Duration::from_secs(60);
    let backend_addr =
        match ports::wait_for_backend_port(args.port, baseline_ports, child_pgid, deadline) {
            Some(addr) => addr,
            None => {
                eprintln!("rack-bridge: server did not listen on loopback within 60s");
                process::kill_child();
                let _ = fs::remove_file(&socket_path);
                std::process::exit(1);
            }
        };

    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("rack-bridge: bind {}: {error}", socket_path.display());
            process::kill_child();
            std::process::exit(1);
        }
    };

    for stream in listener.incoming() {
        let unix_stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };

        thread::spawn(move || tunnel::bridge_to_backend(unix_stream, backend_addr));
    }

    let _ = fs::remove_file(&socket_path);
}
