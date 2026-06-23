use crate::backend::process_metrics::format_bytes;
use crate::backend::BackendCollector;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::time::Duration;

#[test]
fn formats_memory_for_task_manager_readability() {
    assert_eq!(format_bytes(512 * 1024), "512 KB");
    assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2.0 GB");
}

#[test]
fn backend_collects_the_current_process() {
    let tcp = TcpListener::bind("127.0.0.1:0").expect("test TCP listener should bind");
    let udp = UdpSocket::bind("127.0.0.1:0").expect("test UDP socket should bind");
    let tcp_port = tcp.local_addr().unwrap().port();
    let udp_port = udp.local_addr().unwrap().port();
    let mut client = TcpStream::connect(tcp.local_addr().unwrap()).expect("client should connect");
    let (mut server, _) = tcp.accept().expect("server should accept client");
    let mut collector = BackendCollector::new();
    let first = collector.refresh_with_warnings();
    client
        .write_all(&vec![0x5a; 16 * 1024])
        .expect("client upload should succeed");
    let mut uploaded = vec![0_u8; 16 * 1024];
    server
        .read_exact(&mut uploaded)
        .expect("server should receive upload");
    server
        .write_all(&vec![0xa5; 8 * 1024])
        .expect("server download should succeed");
    let mut downloaded = vec![0_u8; 8 * 1024];
    client
        .read_exact(&mut downloaded)
        .expect("client should receive download");
    std::thread::sleep(Duration::from_millis(100));
    let second = collector.refresh_with_warnings();
    let rows = second.rows;
    let current_pid = std::process::id();
    let current = rows
        .iter()
        .find(|row| row.pid == current_pid)
        .expect("current test process should be present");

    assert!(!current.name.is_empty());
    assert!(!current.is_killable);
    assert!(current.last_seen.elapsed().as_secs() < 5);
    assert!(
        current
            .ports
            .iter()
            .any(|binding| binding.local_port == tcp_port),
        "current process TCP listener should be associated by PID"
    );
    assert!(
        current
            .ports
            .iter()
            .any(|binding| binding.local_port == udp_port),
        "current process UDP socket should be associated by PID"
    );

    let counters_unavailable = first
        .warnings
        .iter()
        .chain(second.warnings.iter())
        .any(|warning| warning.contains("TCP byte counters"));
    assert!(
        counters_unavailable || current.upload_bytes > 0 || current.download_bytes > 0,
        "TCP transfer counters should report bytes or an honest availability warning"
    );
}
