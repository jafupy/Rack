use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const HEADER_LIMIT: usize = 64 * 1024;

pub(super) async fn read_request_head(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];

    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);

        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }

        if request.len() > HEADER_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "headers too large",
            ));
        }
    }

    Ok(request)
}

pub(super) fn host_header(request: &[u8]) -> Option<&str> {
    let request = std::str::from_utf8(request).ok()?;
    request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("host").then(|| value.trim())
    })
}

pub(super) async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    message: &str,
) -> io::Result<()> {
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let body = format!("{message}\n");
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\ncontent-type: text/plain; charset=utf-8\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes()).await
}
