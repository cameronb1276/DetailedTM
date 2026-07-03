use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::time::Duration;

use eframe::egui;
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

use crate::backend::{
    self, BackendCollector, BackendRefresh, ProcessPortRow, TrafficCaptureManager,
    TrafficCaptureStatus, TrafficLogPreview,
};

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
    StartTrafficCapture,
    StopTrafficCapture,
}

struct WorkerSnapshot {
    refresh: BackendRefresh,
    refreshed_at: String,
    traffic_status: TrafficCaptureStatus,
}

enum AsyncActionResult {
    EndTask { pid: u32, message: String },
    FileLocation(String),
    TrafficLogs(TrafficLogPreview),
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
    pub traffic_status: TrafficCaptureStatus,
    pub traffic_log_viewer: Option<TrafficLogViewerState>,
    snapshot_rx: Receiver<WorkerSnapshot>,
    command_tx: SyncSender<WorkerCommand>,
    action_tx: Sender<AsyncActionResult>,
    action_rx: Receiver<AsyncActionResult>,
}

pub struct TrafficLogViewerState {
    pub title: String,
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub entries: Vec<String>,
    pub skipped_lines: usize,
    pub errors: Vec<String>,
}

impl AppState {
    pub fn new(context: egui::Context) -> Self {
        let (snapshot_tx, snapshot_rx) = mpsc::sync_channel(2);
        let (command_tx, command_rx) = mpsc::sync_channel(2);
        let (action_tx, action_rx) = mpsc::channel();
        let worker_result = std::thread::Builder::new()
            .name("detailedtm-collector".to_owned())
            .spawn(move || {
                let mut collector = BackendCollector::new();
                let mut traffic = TrafficCaptureManager::new();
                loop {
                    let mut refresh = collector.refresh_with_warnings();
                    if let Err(error) = traffic.record_snapshot(&refresh.rows) {
                        tracing::warn!(%error, "traffic capture snapshot failed");
                        refresh.warnings.push(error.to_string());
                    }

                    let snapshot = WorkerSnapshot {
                        refresh,
                        refreshed_at: local_time_label(),
                        traffic_status: traffic.status(),
                    };
                    if snapshot_tx.send(snapshot).is_err() {
                        break;
                    }
                    context.request_repaint();

                    match command_rx.recv_timeout(REFRESH_INTERVAL) {
                        Ok(command) => handle_worker_command(command, &mut traffic),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }

                    while let Ok(command) = command_rx.try_recv() {
                        handle_worker_command(command, &mut traffic);
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
            traffic_status: TrafficCaptureStatus::stopped(),
            traffic_log_viewer: None,
            snapshot_rx,
            command_tx,
            action_tx,
            action_rx,
        }
    }

    pub fn poll_snapshots(&mut self) {
        let snapshots: Vec<_> = self.snapshot_rx.try_iter().collect();
        let Some(snapshot) = snapshots.into_iter().last() else {
            return;
        };

        self.rows = snapshot.refresh.rows;
        self.last_refresh = snapshot.refreshed_at;
        self.traffic_status = snapshot.traffic_status;
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

    pub fn poll_async_actions(&mut self) {
        let actions: Vec<_> = self.action_rx.try_iter().collect();
        for action in actions {
            match action {
                AsyncActionResult::EndTask { pid, message } => {
                    self.pending_confirmation = None;
                    self.last_status_message = message;
                    if self.selected_pid == Some(pid) {
                        self.request_refresh();
                    }
                }
                AsyncActionResult::FileLocation(message) => {
                    self.last_status_message = message;
                }
                AsyncActionResult::TrafficLogs(preview) => {
                    let process = preview
                        .process_name
                        .clone()
                        .unwrap_or_else(|| format!("PID {}", preview.pid));
                    self.last_status_message = preview.status_message();
                    self.traffic_log_viewer = Some(TrafficLogViewerState {
                        title: format!("Traffic logs: {process}"),
                        root: preview.root,
                        files: preview.files,
                        entries: preview.entries,
                        skipped_lines: preview.skipped_lines,
                        errors: preview.errors,
                    });
                }
            }
        }
    }

    pub fn request_refresh(&self) {
        let _ = self.command_tx.try_send(WorkerCommand::Refresh);
    }

    pub fn start_traffic_capture(&mut self) {
        match self.command_tx.try_send(WorkerCommand::StartTrafficCapture) {
            Ok(()) => {
                self.last_status_message = "Starting traffic capture...".to_owned();
            }
            Err(error) => {
                self.last_status_message = format!("Could not start traffic capture: {error}");
            }
        }
    }

    pub fn stop_traffic_capture(&mut self) {
        match self.command_tx.try_send(WorkerCommand::StopTrafficCapture) {
            Ok(()) => {
                self.last_status_message =
                    "Stopping traffic capture and finalizing logs...".to_owned();
            }
            Err(error) => {
                self.last_status_message = format!("Could not stop traffic capture: {error}");
            }
        }
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
        self.last_status_message = format!("Ending {name} (PID {pid})...");
        let action_tx = self.action_tx.clone();
        std::thread::spawn(move || {
            let message = match backend::kill_process(pid, &name) {
                Ok(()) => {
                    tracing::info!(pid, process = %name, "process ended by user request");
                    format!("Ended {name} (PID {pid})")
                }
                Err(error) => {
                    tracing::warn!(pid, process = %name, %error, "End Task failed");
                    error.to_string()
                }
            };
            let _ = action_tx.send(AsyncActionResult::EndTask { pid, message });
        });
    }

    pub fn open_selected_file_location(&mut self) {
        let Some(row) = self.selected_row() else {
            self.last_status_message = "Select a process first".to_owned();
            return;
        };
        let Some(path) = row.exe_path.clone() else {
            self.last_status_message = format!(
                "Windows did not expose the executable path for {} (PID {})",
                row.name, row.pid
            );
            return;
        };
        let name = row.name.clone();
        let pid = row.pid;
        self.last_status_message = format!("Opening file location for {name} (PID {pid})...");
        let action_tx = self.action_tx.clone();
        std::thread::spawn(move || {
            let message = backend::open_file_location(&path)
                .map(|()| format!("Opened file location for {name} (PID {pid})"))
                .unwrap_or_else(|error| error.to_string());
            let _ = action_tx.send(AsyncActionResult::FileLocation(message));
        });
    }

    pub fn open_selected_traffic_logs(&mut self) {
        let Some(row) = self.selected_row() else {
            self.last_status_message = "Select a process first".to_owned();
            return;
        };
        let pid = row.pid;
        let name = row.name.clone();
        let root = self.traffic_status.log_root.clone();
        self.last_status_message = format!("Opening traffic logs for {name} (PID {pid})...");
        let action_tx = self.action_tx.clone();
        std::thread::spawn(move || {
            let preview = backend::open_traffic_logs(root, pid, Some(name));
            let _ = action_tx.send(AsyncActionResult::TrafficLogs(preview));
        });
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

fn handle_worker_command(command: WorkerCommand, traffic: &mut TrafficCaptureManager) {
    match command {
        WorkerCommand::Refresh => {}
        WorkerCommand::StartTrafficCapture => {
            if let Err(error) = traffic.start() {
                tracing::warn!(%error, "traffic capture could not start");
            }
        }
        WorkerCommand::StopTrafficCapture => {
            if let Err(error) = traffic.stop() {
                tracing::warn!(%error, "traffic capture could not stop cleanly");
            }
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
            exe_path: None,
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
