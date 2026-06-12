use std::ffi::OsString;
use std::path::PathBuf;

pub(super) fn find_wasmtime() -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .chain([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ])
        .map(|dir| dir.join("wasmtime"))
        .find(|candidate| candidate.is_file())
}

pub(super) fn wasmtime_full_wasi_args() -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-S"),
        OsString::from("cli=y"),
        OsString::from("-S"),
        OsString::from("allow-ip-name-lookup=y"),
        OsString::from("-S"),
        OsString::from("tcp=y"),
        OsString::from("-S"),
        OsString::from("udp=y"),
        OsString::from("-S"),
        OsString::from("inherit-env=y"),
    ];
    args.extend(wasi_env_args());
    args
}

fn wasi_env_args() -> Vec<OsString> {
    let mut args = Vec::new();
    for (key, value) in std::env::vars_os() {
        let mut env = key;
        env.push("=");
        env.push(value);
        args.push(OsString::from("--env"));
        args.push(env);
    }
    args
}
