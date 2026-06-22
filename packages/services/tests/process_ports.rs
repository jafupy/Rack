use rack_services::process::parse_listen_ports;

#[test]
fn parses_lsof_listen_port() {
    let output = "node 123 user 20u IPv4 TCP 127.0.0.1:5173 (LISTEN)";

    assert_eq!(parse_listen_ports(output), vec![5173]);
}

#[test]
fn ignores_lines_without_ports() {
    assert!(parse_listen_ports("COMMAND PID USER FD TYPE NAME").is_empty());
}

#[test]
fn dedupes_and_sorts_ports_from_lsof_output() {
    let output = r#"
node 1 user 20u IPv4 TCP 127.0.0.1:3001 (LISTEN)
node 1 user 21u IPv6 TCP [::1]:3000 (LISTEN)
node 1 user 22u IPv4 TCP 127.0.0.1:3001 (LISTEN)
"#;

    assert_eq!(parse_listen_ports(output), vec![3000, 3001]);
}
