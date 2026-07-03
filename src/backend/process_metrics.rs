use std::path::Path;

use sysinfo::{Pid, Process, ProcessesToUpdate, System};

use super::kill;
use super::model::ProcessPortRow;

pub fn refresh(system: &mut System) -> Vec<ProcessPortRow> {
    system.refresh_processes(ProcessesToUpdate::All, true);
    let current_pid = std::process::id();

    system
        .processes()
        .iter()
        .map(|(pid, process)| row_from_process(*pid, process, current_pid))
        .collect()
}

fn row_from_process(pid: Pid, process: &Process, current_pid: u32) -> ProcessPortRow {
    let pid = pid.as_u32();
    let name = executable_name(process);
    let exe_path = process.exe().map(Path::to_path_buf);
    let extension = Path::new(&name)
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ram_usage_bytes = process.memory();

    ProcessPortRow {
        pid,
        name,
        exe_path,
        extension,
        ports: Vec::new(),
        ram_usage_bytes,
        ram_usage_display: format_bytes(ram_usage_bytes),
        cpu_usage_percent: process.cpu_usage(),
        gpu_usage_percent: None,
        is_killable: kill::is_killable(pid, process.name().to_string_lossy().as_ref(), current_pid),
        status: format!("{:?}", process.status()),
        last_seen: std::time::Instant::now(),
    }
}

fn executable_name(process: &Process) -> String {
    process
        .exe()
        .and_then(Path::file_name)
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| process.name().to_string_lossy().into_owned())
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes = bytes as f64;

    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else {
        format!("{:.0} KB", bytes / KB)
    }
}
