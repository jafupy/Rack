mod client;
mod protocol;
mod server;

use std::{io::Write, os::unix::net::UnixStream, path::PathBuf};

use serde::Serialize;

pub use client::Client;
pub use protocol::{Command, Request, Response};
pub use server::ControlServer;

fn write_json_line(stream: &mut UnixStream, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *stream, value).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())
}

fn socket_path() -> PathBuf {
    std::env::var_os("RACK_CONTROL_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/rack.sock"))
}
