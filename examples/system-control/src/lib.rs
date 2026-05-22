use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

#[no_mangle]
pub extern "C" fn system_snapshot() {
    let mut request = String::new();
    let _ = io::stdin().read_to_string(&mut request);

    println!("request_bytes={}", request.len());
    println!("cwd={}", display_result(std::env::current_dir().map(|p| p.display().to_string())));
    println!("home={}", std::env::var("HOME").unwrap_or_else(|_| "<unset>".to_string()));
    println!("user={}", std::env::var("USER").unwrap_or_else(|_| "<unset>".to_string()));
    println!("tmp_entries={}", list_dir("/tmp", 8).join(", "));
    println!("root_entries={}", list_dir("/", 8).join(", "));
}

#[no_mangle]
pub extern "C" fn write_heartbeat() {
    let mut request = String::new();
    let _ = io::stdin().read_to_string(&mut request);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    let line = format!("rack heartbeat at {now}; request_bytes={}\n", request.len());
    match append_line("/tmp/rack-system-control-heartbeat.log", &line) {
        Ok(()) => println!("wrote /tmp/rack-system-control-heartbeat.log"),
        Err(error) => println!("failed to write heartbeat: {error}"),
    }
}

fn list_dir(path: &str, limit: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(path) else {
        return vec![format!("<cannot read {path}>")];
    };

    let mut names = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .take(limit)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn append_line(path: &str, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())
}

fn display_result(result: io::Result<String>) -> String {
    result.unwrap_or_else(|error| format!("<error: {error}>"))
}
