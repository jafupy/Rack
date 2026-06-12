use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(serde::Deserialize)]
struct InferPayload {
    directory: PathBuf,
    #[serde(default, rename = "standardPortsEnabled")]
    standard_ports_enabled: bool,
    #[serde(default, rename = "proxyPort")]
    proxy_port: u16,
}

#[derive(Serialize)]
struct InferredProject {
    name: String,
    #[serde(rename = "sanitizedName")]
    sanitized_name: String,
    #[serde(rename = "localURL")]
    local_url: String,
}

pub(crate) fn project_command(command_type: &str, payload: &Value) -> Option<Value> {
    if command_type != "project.infer" {
        return None;
    }

    Some(match infer_command(payload) {
        Ok(project) => serde_json::json!({
            "type": "project.infer",
            "payload": project,
        }),
        Err(message) => serde_json::json!({
            "type": "error",
            "message": message,
        }),
    })
}

fn infer_command(payload: &Value) -> Result<InferredProject, String> {
    let payload: InferPayload =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    Ok(infer(
        &payload.directory,
        payload.standard_ports_enabled,
        payload.proxy_port,
    ))
}

fn infer(directory: &Path, standard_ports_enabled: bool, proxy_port: u16) -> InferredProject {
    let base = base_name(directory);
    let name = match worktree_branch(directory) {
        Some(branch) => {
            let segment = branch.rsplit('/').next().unwrap_or(&branch);
            format!("{}.{}", sanitize(segment), sanitize(&base))
        }
        None => sanitize(&base),
    };

    let port_suffix = if standard_ports_enabled {
        String::new()
    } else {
        format!(":{proxy_port}")
    };
    InferredProject {
        name: name.clone(),
        sanitized_name: name.clone(),
        local_url: format!("http://{name}.localhost{port_suffix}"),
    }
}

fn base_name(directory: &Path) -> String {
    if let Some(remote) = git_output(directory, &["remote", "get-url", "origin"]) {
        let stripped = remote
            .trim()
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".git");
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }

    let package_json = directory.join("package.json");
    if let Ok(source) = std::fs::read_to_string(package_json) {
        if let Ok(json) = serde_json::from_str::<Value>(&source) {
            if let Some(name) = json.get("name").and_then(Value::as_str) {
                if !name.is_empty() {
                    return name.rsplit('/').next().unwrap_or(name).to_string();
                }
            }
        }
    }

    directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app")
        .to_string()
}

fn worktree_branch(directory: &Path) -> Option<String> {
    let raw = git_output(directory, &["worktree", "list", "--porcelain"])?;
    let worktrees = parse_worktrees(&raw);
    let main = worktrees.first()?;
    let current_path = real_path(directory);
    let main_path = real_path(&main.path);
    if current_path == main_path {
        return None;
    }

    worktrees
        .into_iter()
        .find(|worktree| current_path.starts_with(&real_path(&worktree.path)))
        .and_then(|worktree| worktree.branch)
}

struct Worktree {
    path: PathBuf,
    branch: Option<String>,
}

fn parse_worktrees(raw: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;

    for line in raw.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(path) = path.take() {
                worktrees.push(Worktree {
                    path,
                    branch: branch.take(),
                });
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        } else if let Some(value) = line.strip_prefix("branch ") {
            branch = Some(value.trim_start_matches("refs/heads/").to_string());
        }
    }

    worktrees
}

fn git_output(directory: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn sanitize(name: &str) -> String {
    name.to_ascii_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|ch| !"^@#$%&*()[]{}|;:',.<>?/\\\"".contains(*ch))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_package_names() {
        assert_eq!(sanitize("@scope/My App!"), "scopemy-app!");
    }

    #[test]
    fn parses_worktree_branches() {
        let raw = "worktree /repo\nbranch refs/heads/main\n\nworktree /repo-linked\nbranch refs/heads/feature/auth\n\n";
        let worktrees = parse_worktrees(raw);
        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[1].branch.as_deref(), Some("feature/auth"));
    }

    #[test]
    fn builds_local_url_with_port_when_standard_ports_are_disabled() {
        let project = infer(Path::new("/tmp/My App"), false, 1355);
        assert_eq!(project.name, "my-app");
        assert_eq!(project.local_url, "http://my-app.localhost:1355");
    }
}
