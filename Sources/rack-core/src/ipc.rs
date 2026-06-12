use crate::config::handle_ipc_message_with_current_context;
use crate::{emit, EventCallback};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub(crate) fn start_ipc_server(
    stop: Arc<AtomicBool>,
    callback: Option<EventCallback>,
    context: usize,
) {
    std::thread::spawn(move || {
        let path = socket_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&path);

        let listener = match UnixListener::bind(&path) {
            Ok(listener) => listener,
            Err(error) => {
                if let Some(callback) = callback {
                    emit(
                        callback,
                        context,
                        &serde_json::json!({
                            "type": "ipc.error",
                            "payload": error.to_string(),
                        })
                        .to_string(),
                    );
                }
                return;
            }
        };
        let _ = listener.set_nonblocking(true);

        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let callback = callback;
                    std::thread::spawn(move || handle_client(stream, callback, context));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }

        let _ = std::fs::remove_file(path);
    });
}

fn handle_client(mut stream: UnixStream, callback: Option<EventCallback>, context: usize) {
    let Some(line) = read_line(&mut stream) else {
        let _ = stream.write_all(b"{\"type\":\"error\",\"payload\":\"invalid message\"}\n");
        return;
    };

    let message: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
    let response = handle_ipc_message_with_current_context(&message);
    let result = response.get("payload").unwrap_or(&Value::Null);
    let reply = result.get("reply").unwrap_or(&Value::Null);
    let mut reply = reply.to_string();
    reply.push('\n');
    let _ = stream.write_all(reply.as_bytes());

    if let Some(action) = result.get("action").filter(|action| !action.is_null()) {
        if let Some(callback) = callback {
            emit(
                callback,
                context,
                &serde_json::json!({
                    "type": "ipc.action",
                    "payload": action,
                })
                .to_string(),
            );
        }
    }
}

fn read_line(stream: &mut UnixStream) -> Option<String> {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = stream.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..count]);
        if data.contains(&b'\n') {
            break;
        }
    }

    let line = data.split(|byte| *byte == b'\n').next()?;
    String::from_utf8(line.to_vec()).ok()
}

fn socket_path() -> PathBuf {
    home_dir().join(".config").join("rack").join("rack.sock")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
