mod dev;
mod function_cli;

use serde_json::{json, Value};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub(crate) type Result<T> = std::result::Result<T, String>;

fn main() {
    if let Err(error) = run() {
        eprintln!("rack: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("dev") => dev::cmd_dev(),
        Some("ls" | "list") => cmd_ls(),
        Some("start") => {
            let name = args.get(1).ok_or("Usage: rack start <name>")?;
            send(&json!({ "type": "start", "payload": name }))?;
            println!("✓ started {name}");
            Ok(())
        }
        Some("stop") => {
            let name = args.get(1).ok_or("Usage: rack stop <name>")?;
            send(&json!({ "type": "stop", "payload": name }))?;
            println!("✓ stopped {name}");
            Ok(())
        }
        Some("rm" | "remove") => {
            let name = args.get(1).ok_or("Usage: rack rm <name>")?;
            send(&json!({ "type": "remove", "payload": name }))?;
            println!("✓ removed {name}");
            Ok(())
        }
        Some("fn") => function_cli::cmd_function(&args[1..]),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn cmd_ls() -> Result<()> {
    let reply = send(&json!({ "type": "list" }))?;
    let Some(servers) = reply.get("payload").and_then(Value::as_array) else {
        println!("No servers registered. Run 'rack dev' in a project directory.");
        return Ok(());
    };
    if servers.is_empty() {
        println!("No servers registered. Run 'rack dev' in a project directory.");
        return Ok(());
    }

    let name_width = servers
        .iter()
        .filter_map(|server| server.get("name").and_then(Value::as_str))
        .map(str::len)
        .max()
        .unwrap_or(4);
    println!("{}", "─".repeat(name_width + 40));
    for server in servers {
        let name = server.get("name").and_then(Value::as_str).unwrap_or("");
        let url = server.get("url").and_then(Value::as_str).unwrap_or("");
        let running = server
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let dot = if running { "●" } else { "○" };
        println!("{dot}  {name:<name_width$}  {url}");
    }
    println!("{}", "─".repeat(name_width + 40));
    Ok(())
}

pub(crate) fn send(message: &Value) -> Result<Value> {
    let socket_path = home_dir().join(".config/rack/rack.sock");
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|_| "Rack.app is not running — open it first".to_string())?;
    stream
        .write_all(message.to_string().as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| error.to_string())?;

    let mut reply = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream.read(&mut byte).map_err(|error| error.to_string())?;
        if read == 0 || byte[0] == b'\n' {
            break;
        }
        reply.push(byte[0]);
    }

    serde_json::from_slice(&reply).map_err(|error| error.to_string())
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn print_usage() {
    println!("rack — dev environment manager");
    println!();
    println!("  rack dev                    Register this directory with Rack.app");
    println!("  rack fn add [path]          Build and install a Rust function package");
    println!("  rack fn init [path]         Create a Rust/WASI function package");
    println!("  rack fn compile [path]      Build a Rust function package");
    println!("  rack fn test [path] [fn]    Compile and run a function locally");
    println!("  rack fn install [path] [--replace] [--link]");
    println!("  rack fn ls                  List installed functions");
    println!("  rack fn rm <name>           Remove an installed function");
    println!("  rack ls                     List registered servers");
    println!("  rack start <name>           Start a server");
    println!("  rack stop <name>            Stop a server");
    println!("  rack rm <name>              Remove a server");
    println!();
    println!("Run 'rack dev' in a project directory. Rack.app must be running.");
}
