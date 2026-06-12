use crate::{emit, EventCallback};
use std::io::{BufReader, Read};
use std::thread;

pub(super) fn spawn_output_reader(
    id: String,
    stream: impl Read + Send + 'static,
    callback: Option<EventCallback>,
    context: usize,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buffer = [0u8; 4096];
        loop {
            let Ok(count) = reader.read(&mut buffer) else {
                break;
            };
            if count == 0 {
                break;
            }
            let output = String::from_utf8_lossy(&buffer[..count]).to_string();
            emit_output(callback, context, &id, &output);
        }
    });
}

pub(super) fn emit_output(callback: Option<EventCallback>, context: usize, id: &str, output: &str) {
    if let Some(callback) = callback {
        emit(
            callback,
            context,
            &serde_json::json!({
                "type": "server.output",
                "payload": {
                    "id": id,
                    "output": output,
                }
            })
            .to_string(),
        );
    }
}
