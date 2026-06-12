use std::env;
use std::path::PathBuf;

pub(crate) struct Args {
    pub(crate) socket_path: PathBuf,
    pub(crate) port: u16,
    pub(crate) command: String,
    pub(crate) command_args: Vec<String>,
}

pub(crate) fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = env::args().skip(1).collect();
    let mut socket_path = None;
    let mut port = None;
    let mut i = 0;

    while i < raw.len() {
        match raw[i].as_str() {
            "--socket" => {
                i += 1;
                socket_path = Some(PathBuf::from(raw.get(i).ok_or("--socket needs a value")?));
            }
            "--port" => {
                i += 1;
                port = Some(
                    raw.get(i)
                        .ok_or("--port needs a value")?
                        .parse::<u16>()
                        .map_err(|_| "--port must be a number")?,
                );
            }
            "--" => {
                i += 1;
                let rest = raw.get(i..).unwrap_or(&[]);
                return Ok(Args {
                    socket_path: socket_path.ok_or("--socket is required")?,
                    port: port.ok_or("--port is required")?,
                    command: rest
                        .first()
                        .cloned()
                        .ok_or("command is required after --")?,
                    command_args: rest.iter().skip(1).cloned().collect(),
                });
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        i += 1;
    }

    Err("usage: rack-bridge --socket <path> --port <n> -- <command> [args...]".into())
}
