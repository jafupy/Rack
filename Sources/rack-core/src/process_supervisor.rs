use crate::process::{launch_plan, LaunchPlan, LaunchPlanRequest};
use crate::{emit, EventCallback};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
struct RunningProcess {
    pid: u32,
    pgid: i32,
    plan: LaunchPlan,
}

static PROCESSES: OnceLock<Mutex<HashMap<String, RunningProcess>>> = OnceLock::new();

fn processes() -> &'static Mutex<HashMap<String, RunningProcess>> {
    PROCESSES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn supervisor_command(
    command_type: &str,
    payload: &Value,
    callback: Option<EventCallback>,
    context: usize,
) -> Option<Value> {
    let response = match command_type {
        "server.start" => start_server(payload, callback, context),
        "server.stop" => stop_server(payload),
        _ => return None,
    };
    Some(response.unwrap_or_else(|message| {
        serde_json::json!({
            "type": "error",
            "message": message,
        })
    }))
}

fn start_server(
    payload: &Value,
    callback: Option<EventCallback>,
    context: usize,
) -> Result<Value, String> {
    let request: LaunchPlanRequest =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    let id = request.config.id.clone();
    if processes().lock().unwrap().contains_key(&id) {
        return Err("server is already running".to_string());
    }

    let plan = launch_plan(payload)?;
    let mut command = Command::new(&plan.executable);
    command.args(&plan.arguments);
    if !plan.working_directory.is_empty() {
        command.current_dir(&plan.working_directory);
    }
    command.env_clear();
    command.envs(&plan.environment);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id();
    let pgid = pid as i32;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    processes().lock().unwrap().insert(
        id.clone(),
        RunningProcess {
            pid,
            pgid,
            plan: plan.clone(),
        },
    );

    emit_output(
        callback,
        context,
        &id,
        &format!("Launching: {}\n", plan.launch_description),
    );
    if !plan.working_directory.is_empty() {
        emit_output(
            callback,
            context,
            &id,
            &format!("Working directory: {}\n", plan.working_directory),
        );
    }

    if let Some(stdout) = stdout {
        spawn_output_reader(id.clone(), stdout, callback, context);
    }
    if let Some(stderr) = stderr {
        spawn_output_reader(id.clone(), stderr, callback, context);
    }

    let exit_plan = plan.clone();
    thread::spawn(move || {
        let status = child.wait().ok();
        processes().lock().unwrap().remove(&id);
        if let Some(callback) = callback {
            emit(
                callback,
                context,
                &serde_json::json!({
                    "type": "server.exited",
                    "payload": {
                        "id": id,
                        "status": status.and_then(|status| status.code()).unwrap_or(-1),
                        "plan": exit_plan,
                    }
                })
                .to_string(),
            );
        }
    });

    Ok(serde_json::json!({
        "type": "server.started",
        "payload": {
            "pid": pid,
            "plan": plan,
        }
    }))
}

fn stop_server(payload: &Value) -> Result<Value, String> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing server id".to_string())?;
    let process = processes().lock().unwrap().get(id).cloned();
    if let Some(process) = process {
        unsafe {
            libc::kill(-process.pgid, libc::SIGTERM);
        }
        thread::sleep(Duration::from_millis(300));
        unsafe {
            libc::kill(-process.pgid, libc::SIGKILL);
        }
        Ok(serde_json::json!({
            "type": "server.stopped",
            "payload": {
                "id": id,
                "pid": process.pid,
                "plan": process.plan,
            }
        }))
    } else {
        Ok(serde_json::json!({
            "type": "server.stopped",
            "payload": {
                "id": id,
                "pid": null,
                "plan": null,
            }
        }))
    }
}

fn spawn_output_reader(
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

fn emit_output(callback: Option<EventCallback>, context: usize, id: &str, output: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_starts_process_and_returns_pid() {
        let payload = serde_json::json!({
            "config": {
                "id": "00000000-0000-4000-8000-000000000002",
                "name": "Echo App",
                "command": "/bin/echo",
                "arguments": "hello",
                "workingDirectory": "",
                "environment": []
            },
            "context": {}
        });

        let response = supervisor_command("server.start", &payload, None, 0).unwrap();
        assert_eq!(response["type"], "server.started");
        assert!(response["payload"]["pid"].as_u64().unwrap() > 0);
    }
}
