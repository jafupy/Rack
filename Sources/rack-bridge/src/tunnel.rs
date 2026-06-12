use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::net::UnixStream;
use std::thread;

fn bridge(unix: UnixStream, tcp: TcpStream) {
    let unix_read = unix.try_clone().expect("clone unix stream");
    let tcp_read = tcp.try_clone().expect("clone tcp stream");

    let a2b = thread::spawn(move || {
        let mut src = unix_read;
        let mut dst = tcp;
        let mut buf = vec![0u8; 65536];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if dst.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = dst.shutdown(std::net::Shutdown::Write);
    });

    let b2a = thread::spawn(move || {
        let mut src = tcp_read;
        let mut dst = unix;
        let mut buf = vec![0u8; 65536];
        loop {
            match src.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if dst.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = dst.shutdown(std::net::Shutdown::Both);
    });

    let _ = a2b.join();
    let _ = b2a.join();
}

pub(crate) fn bridge_to_backend(unix_stream: UnixStream, backend_addr: SocketAddr) {
    match TcpStream::connect(backend_addr) {
        Ok(tcp_stream) => {
            let _ = tcp_stream.set_nodelay(true);
            bridge(unix_stream, tcp_stream);
        }
        Err(error) => {
            let mut stream = unix_stream;
            let body = format!("rack-bridge: backend not reachable: {error}");
            let response = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    }
}
