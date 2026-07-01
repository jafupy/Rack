use std::collections::HashMap;

use crate::process::Process;

use super::log::append_service_log;

const MAX_LOG_LINES: usize = 400;

pub(super) fn collect_output(
    processes: &mut HashMap<String, Process>,
    logs: &mut HashMap<String, String>,
) {
    for (id, process) in processes {
        append_output(logs, id, process.drain_output());
    }
}

pub(super) fn append_output(logs: &mut HashMap<String, String>, id: &str, output: Vec<String>) {
    if output.is_empty() {
        return;
    }

    append_service_log(id, &output);

    let log = logs.entry(id.to_string()).or_default();
    for chunk in output {
        log.push_str(&chunk);
    }

    let lines = log.lines().count();
    if lines > MAX_LOG_LINES {
        *log = log
            .lines()
            .skip(lines - MAX_LOG_LINES)
            .collect::<Vec<_>>()
            .join("\n");
    }
}
