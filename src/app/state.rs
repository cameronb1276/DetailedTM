use std::cmp::Ordering;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

use eframe::egui;
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

use crate::backend::{self, BackendCollector, BackendRefresh, ProcessPortRow};

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchMode {
    Name,
    Pid,
    Port,
    Extension,
}

impl SearchMode {
    pub const ALL: [Self; 4] = [Self::Name, Self::Pid, Self::Port, Self::Extension];

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Pid => "PID",
            Self::Port => "PORT",
            Self::Extension => "extension",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortColumn {
    Pid,
    Name,
    Port,
    Ram,
    Cpu,
    Gpu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn marker(self) -> &'static str {
        match self {
            Self::Ascending => " ▲",
            Self::Descending => " ▼",
        }
    }
}

enum WorkerCommand {
    Refresh,
}

struct WorkerSnapshot {
    refresh: BackendRefresh,
    refreshed_at: String,
}

pub struct AppState {
    pub rows: Vec<ProcessPortRow>,
    pub search: String,
    pub search_mode: SearchMode,
    pub selected_pid: Option<u32>,
    pub sort_column: Option<SortColumn>,
    pub sort_direction: SortDirection,
    pub last_refresh: String,
    pub last_status_message: String,
    pub backend_warning: Option<String>,
    pub pending_confirmation: Option<u32>,
    snapshot_rx: Receiver<WorkerSnapshot>,
    command_tx: SyncSender<WorkerCommand>,
}

impl AppState {
    pub fn new(context: egui::Context) -> Self {
        let (snapshot_tx, snapshot_rx) = mpsc::sync_channel(2);
        let (command_tx, command_rx) = mpsc::sync_channel(2);
        let worker_result = std::thread::Builder::new()
            .name("detailedtm-collector".to_owned())
            .spawn(move || {
                let mut collector = BackendCollector::new();
                loop {
                    let snapshot = WorkerSnapshot {
                        refresh: collector.refresh_with_warnings(),
                        refreshed_at: local_time_label(),
                    };
                    if snapshot_tx.send(snapshot).is_err() {
                        break;
                    }
                    context.request_repaint();

                    match command_rx.recv_timeout(REFRESH_INTERVAL) {
                        Ok(WorkerCommand::Refresh) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });

        let (last_refresh, last_status_message) = match worker_result {
            Ok(_) => (
                "Waiting for first snapshot…".to_owned(),
                "Collecting process data…".to_owned(),
            ),
            Err(error) => {
                tracing::error!(%error, "background collector thread could not start");
                (
                    "Unavailable".to_owned(),
                    format!("Process collector could not start: {error}"),
                )
            }
        };

        Self {
            rows: Vec::new(),
            search: String::new(),
            search_mode: SearchMode::Name,
            selected_pid: None,
            sort_column: None,
            sort_direction: SortDirection::Ascending,
            last_refresh,
            last_status_message,
            backend_warning: None,
            pending_confirmation: None,
            snapshot_rx,
            command_tx,
        }
    }

    pub fn poll_snapshots(&mut self) {
        let snapshots: Vec<_> = self.snapshot_rx.try_iter().collect();
        let Some(snapshot) = snapshots.into_iter().last() else {
            return;
        };

        self.rows = snapshot.refresh.rows;
        self.last_refresh = snapshot.refreshed_at;
        self.backend_warning =
            (!snapshot.refresh.warnings.is_empty()).then(|| snapshot.refresh.warnings.join(" | "));

        if let Some(pid) = self.selected_pid {
            if !self.rows.iter().any(|row| row.pid == pid) {
                self.selected_pid = None;
                self.pending_confirmation = None;
                self.last_status_message =
                    format!("PID {pid} is no longer running; selection cleared");
            }
        }
    }

    pub fn request_refresh(&self) {
        let _ = self.command_tx.try_send(WorkerCommand::Refresh);
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| matches_search(row, self.search_mode, &self.search))
            .map(|(index, _)| index)
            .collect();

        if let Some(column) = self.sort_column {
            indices.sort_by(|left, right| {
                let ordering = compare_rows(&self.rows[*left], &self.rows[*right], column);
                match self.sort_direction {
                    SortDirection::Ascending => ordering,
                    SortDirection::Descending => ordering.reverse(),
                }
            });
        }
        indices
    }

    pub fn select(&mut self, pid: u32) {
        self.selected_pid = Some(pid);
        self.last_status_message = format!("Selected PID {pid}");
    }

    pub fn selected_row(&self) -> Option<&ProcessPortRow> {
        let pid = self.selected_pid?;
        self.rows.iter().find(|row| row.pid == pid)
    }

    pub fn begin_end_task(&mut self) {
        if let Some(row) = self.selected_row().filter(|row| row.is_killable) {
            self.pending_confirmation = Some(row.pid);
        }
    }

    pub fn cancel_end_task(&mut self) {
        self.pending_confirmation = None;
    }

    pub fn confirm_end_task(&mut self) {
        let Some(pid) = self.pending_confirmation.take() else {
            return;
        };
        let Some(row) = self.rows.iter().find(|row| row.pid == pid) else {
            self.last_status_message = format!("PID {pid} is no longer running");
            self.selected_pid = None;
            return;
        };
        let name = row.name.clone();
        match backend::kill_process(pid, &name) {
            Ok(()) => {
                tracing::info!(pid, process = %name, "process ended by user request");
                self.last_status_message = format!("Ended {name} (PID {pid})");
            }
            Err(error) => {
                tracing::warn!(pid, process = %name, %error, "End Task failed");
                self.last_status_message = error.to_string();
            }
        }
        self.request_refresh();
    }

    pub fn set_sort(&mut self, column: SortColumn) {
        if self.sort_column == Some(column) {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = Some(column);
            self.sort_direction = match column {
                SortColumn::Ram | SortColumn::Cpu | SortColumn::Gpu => SortDirection::Descending,
                _ => SortDirection::Ascending,
            };
        }
    }

    pub fn header_label(&self, label: &str, column: SortColumn) -> String {
        if self.sort_column == Some(column) {
            format!("{label}{}", self.sort_direction.marker())
        } else {
            label.to_owned()
        }
    }
}

fn compare_rows(left: &ProcessPortRow, right: &ProcessPortRow, column: SortColumn) -> Ordering {
    match column {
        SortColumn::Pid => left.pid.cmp(&right.pid),
        SortColumn::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        SortColumn::Port => first_port(left).cmp(&first_port(right)),
        SortColumn::Ram => left.ram_usage_bytes.cmp(&right.ram_usage_bytes),
        SortColumn::Cpu => left
            .cpu_usage_percent
            .partial_cmp(&right.cpu_usage_percent)
            .unwrap_or(Ordering::Equal),
        SortColumn::Gpu => left
            .gpu_usage_percent
            .partial_cmp(&right.gpu_usage_percent)
            .unwrap_or(Ordering::Less),
    }
}

fn first_port(row: &ProcessPortRow) -> u16 {
    row.ports
        .iter()
        .map(|binding| binding.local_port)
        .min()
        .unwrap_or(u16::MAX)
}

fn matches_search(row: &ProcessPortRow, mode: SearchMode, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    match mode {
        SearchMode::Name => needle.is_empty() || row.name.to_lowercase().contains(&needle),
        SearchMode::Pid => needle.is_empty() || row.pid.to_string().contains(&needle),
        SearchMode::Port => {
            needle.is_empty()
                || row
                    .ports
                    .iter()
                    .any(|port| port.local_port.to_string().contains(&needle))
        }
        SearchMode::Extension => {
            let needle = needle.trim_start_matches('.');
            needle.is_empty() || row.extension.to_lowercase().contains(needle)
        }
    }
}

fn local_time_label() -> String {
    let mut time = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut time) };
    let (hour, suffix) = match time.wHour {
        0 => (12, "AM"),
        1..=11 => (time.wHour, "AM"),
        12 => (12, "PM"),
        _ => (time.wHour - 12, "PM"),
    };
    format!("{hour}:{:02}:{:02} {suffix}", time.wMinute, time.wSecond)
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::time::Instant;

    use crate::backend::model::{PortBinding, Protocol, TcpState};

    use super::{matches_search, ProcessPortRow, SearchMode};

    fn sample_row() -> ProcessPortRow {
        ProcessPortRow {
            pid: 12345,
            name: "Example.EXE".to_owned(),
            extension: "EXE".to_owned(),
            ports: vec![PortBinding {
                pid: 12345,
                protocol: Protocol::Tcp,
                local_addr: Ipv4Addr::LOCALHOST,
                local_port: 5173,
                remote_addr: None,
                remote_port: None,
                state: TcpState::Listening,
            }],
            ram_usage_bytes: 1,
            ram_usage_display: "1 KB".to_owned(),
            cpu_usage_percent: 0.0,
            gpu_usage_percent: None,
            is_killable: true,
            status: "Run".to_owned(),
            last_seen: Instant::now(),
        }
    }

    #[test]
    fn search_modes_match_expected_fields() {
        let row = sample_row();
        assert!(matches_search(&row, SearchMode::Name, "example"));
        assert!(matches_search(&row, SearchMode::Pid, "234"));
        assert!(matches_search(&row, SearchMode::Port, "517"));
        assert!(matches_search(&row, SearchMode::Extension, ".exe"));
        assert!(!matches_search(&row, SearchMode::Name, "missing"));
    }
}
