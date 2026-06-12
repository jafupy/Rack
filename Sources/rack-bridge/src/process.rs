use crate::args::Args;
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

static CHILD_PGID: AtomicI32 = AtomicI32::new(0);

pub(crate) fn kill_child() {
    let pgid = CHILD_PGID.load(Ordering::SeqCst);
    if pgid > 0 {
        unsafe { libc::kill(-pgid, libc::SIGTERM) };
        thread::sleep(Duration::from_millis(300));
        unsafe { libc::kill(-pgid, libc::SIGKILL) };
    }
}

pub(crate) fn setup_signals(socket_path: PathBuf) {
    for sig in [libc::SIGTERM, libc::SIGINT] {
        let path = socket_path.clone();
        thread::spawn(move || unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, sig);
            let mut received = 0;
            libc::sigwait(&set, &mut received);
            kill_child();
            let _ = fs::remove_file(&path);
            std::process::exit(0);
        });
    }
}

fn rack_search_paths() -> Vec<PathBuf> {
    let mut search_paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        search_paths.extend([
            home.join(".bun/bin"),
            home.join(".local/bin"),
            home.join(".cargo/bin"),
        ]);
    }
    search_paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/opt/homebrew/sbin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ]);

    search_paths
}

fn rack_child_path() -> Option<String> {
    env::join_paths(rack_search_paths())
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn resolve_command(command: &str) -> String {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command.to_string();
    }

    rack_search_paths()
        .into_iter()
        .map(|dir| dir.join(command))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| command.to_string())
}

pub(crate) fn block_termination_signals() {
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    unsafe {
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut mask);
        libc::sigaddset(&mut mask, libc::SIGTERM);
        libc::sigaddset(&mut mask, libc::SIGINT);
        libc::pthread_sigmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut());
    }
}

pub(crate) fn spawn_child(args: &Args) -> Result<(Child, i32), String> {
    let command_path = resolve_command(&args.command);
    let mut child = Command::new(&command_path);
    if let Some(path) = rack_child_path() {
        child.env("PATH", path);
    }
    child
        .args(&args.command_args)
        .env("PORT", args.port.to_string())
        .env("HOST", "127.0.0.1");

    unsafe {
        child.pre_exec(move || {
            libc::setpgid(0, 0);
            let mut mask: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut mask);
            libc::sigprocmask(libc::SIG_SETMASK, &mask, std::ptr::null_mut());
            Ok(())
        });
    }

    let child = child
        .spawn()
        .map_err(|error| format!("failed to spawn '{}': {error}", args.command))?;
    let child_pgid = child.id() as i32;
    CHILD_PGID.store(child_pgid, Ordering::SeqCst);
    Ok((child, child_pgid))
}

pub(crate) fn reap_child(mut child: Child, socket_path: PathBuf) {
    thread::spawn(move || {
        let code = child
            .wait()
            .ok()
            .and_then(|status| status.code())
            .unwrap_or(1);
        let _ = fs::remove_file(&socket_path);
        std::process::exit(code);
    });
}
