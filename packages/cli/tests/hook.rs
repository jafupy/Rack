use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn hook_list_reports_empty_home() {
    let home = temp_home("hook-list-empty");

    let output = rack(&home, ["hook", "list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "No hooks deployed\n");
}

#[test]
fn hook_ls_lists_deployed_hook_errors() {
    let home = temp_home("hook-ls");
    fs::create_dir_all(home.join(".rack/hooks/example")).unwrap();

    let output = rack(&home, ["hook", "ls"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "example\n  error\tbuilt wasm not found\n");
}

#[test]
fn hook_test_help_exposes_selectors() {
    let home = temp_home("hook-test-help");

    let output = rack(&home, ["hook", "test", "--help"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("--hook"));
    assert!(stdout(&output).contains("--route"));
}

#[test]
fn hook_remove_deletes_deployed_hook() {
    let home = temp_home("hook-remove");
    let deployed = home.join(".rack/hooks/example");
    fs::create_dir_all(&deployed).unwrap();

    let output = rack(&home, ["hook", "remove", "example"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!deployed.exists());
    assert!(stdout(&output).contains("Removed hook `example`"));
}

#[test]
fn hook_rm_rejects_path_traversal() {
    let home = temp_home("hook-rm-rejects-path");
    fs::create_dir_all(home.join(".rack/hooks/example")).unwrap();

    let output = rack(&home, ["hook", "rm", "../example"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("hook name must be a deployed hook directory name"));
}

fn rack<const N: usize>(home: &Path, args: [&str; N]) -> Output {
    Command::new(std::env::var("CARGO_BIN_EXE_rack-cli").unwrap())
        .args(args)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn temp_home(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rack-cli-{name}-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
