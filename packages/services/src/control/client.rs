use std::{
    io::{BufRead, BufReader},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use super::{write_json_line, Request, Response};

pub struct Client {
    path: PathBuf,
}
impl Client {
    pub fn connect_default() -> Self {
        Self {
            path: super::socket_path(),
        }
    }

    pub fn request(&self, request: Request) -> Result<Response, String> {
        let mut stream = UnixStream::connect(&self.path)
            .map_err(|error| format!("rack is not running at {}: {error}", self.path.display()))?;
        write_json_line(&mut stream, &request)?;

        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&line).map_err(|error| error.to_string())
    }
}
