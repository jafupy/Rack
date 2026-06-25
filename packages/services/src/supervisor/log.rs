use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

pub(super) fn clear_service_log(id: &str) {
    let path = service_log_path(id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, "");
}

pub(super) fn append_service_log(id: &str, chunks: &[String]) {
    if chunks.is_empty() {
        return;
    }

    let path = service_log_path(id);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    for chunk in chunks {
        let _ = file.write_all(chunk.as_bytes());
    }
}

pub fn service_log_path(id: &str) -> PathBuf {
    rack_home()
        .join("logs/services")
        .join(format!("{}.log", safe_file_name(id)))
}

fn rack_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rack")
}

fn safe_file_name(value: &str) -> String {
    value
        .chars()
        .map(|char| match char {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => char,
            _ => '_',
        })
        .collect()
}
