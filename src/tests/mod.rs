use crate::backend::process_metrics::format_bytes;
use crate::backend::BackendCollector;
use std::net::{TcpListener, UdpSocket};

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
    let mut collector = BackendCollector::new();
    let rows = collector.refresh_with_warnings().rows;
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
}
