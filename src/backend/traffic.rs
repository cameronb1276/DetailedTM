use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use chrono::{Local, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use super::model::{PortBinding, ProcessPortRow, Protocol, TcpState};

const MAX_RAW_LOG_BYTES: u64 = 25 * 1024 * 1024;
const MAX_PREVIEW_ENTRIES: usize = 1_000;

#[derive(Clone, Debug)]
pub struct TrafficCaptureStatus {
    pub running: bool,
    pub log_root: PathBuf,
    pub last_message: String,
}

impl TrafficCaptureStatus {
    pub fn stopped() -> Self {
        Self {
            running: false,
            log_root: default_log_root(),
            last_message: "Traffic capture stopped".to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct TrafficLogPreview {
    pub pid: u32,
    pub process_name: Option<String>,
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub entries: Vec<String>,
    pub skipped_lines: usize,
    pub errors: Vec<String>,
}

impl TrafficLogPreview {
    pub fn status_message(&self) -> String {
        if self.files.is_empty() {
            return "No traffic logs found for this process.".to_owned();
        }
        if self.entries.is_empty() && !self.errors.is_empty() {
            return format!(
                "Traffic logs were found, but no readable entries could be loaded: {}",
                self.errors.join(" | ")
            );
        }
        format!(
            "Loaded {} traffic log entries from {} file(s); skipped {} malformed line(s)",
            self.entries.len(),
            self.files.len(),
            self.skipped_lines
        )
    }
}

#[derive(Debug, Error)]
pub enum TrafficLogError {
    #[error("Could not create traffic log folder {path}: {source}")]
    CreateFolder {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Could not create traffic log {path}: {source}")]
    CreateFile {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Could not write traffic log {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Could not serialize traffic event for PID {pid}: {source}")]
    Serialize {
        pid: u32,
        #[source]
        source: serde_json::Error,
    },
    #[error("Could not finalize traffic log {path}: {source}")]
    Finalize {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Could not compress traffic log {path}: {source}")]
    Compress {
        path: String,
        #[source]
        source: io::Error,
    },
}

pub struct TrafficCaptureManager {
    running: bool,
    root: PathBuf,
    writers: HashMap<LogKey, ProcessLogWriter>,
    last_message: String,
}

impl TrafficCaptureManager {
    pub fn new() -> Self {
        Self {
            running: false,
            root: default_log_root(),
            writers: HashMap::new(),
            last_message: "Traffic capture stopped".to_owned(),
        }
    }

    pub fn status(&self) -> TrafficCaptureStatus {
        TrafficCaptureStatus {
            running: self.running,
            log_root: self.root.clone(),
            last_message: self.last_message.clone(),
        }
    }

    pub fn start(&mut self) -> Result<(), TrafficLogError> {
        fs::create_dir_all(&self.root).map_err(|source| TrafficLogError::CreateFolder {
            path: self.root.display().to_string(),
            source,
        })?;
        self.running = true;
        self.last_message = format!("Traffic capture running; logs: {}", self.root.display());
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), TrafficLogError> {
        self.running = false;
        let mut first_error = None;
        for (_, writer) in self.writers.drain() {
            if let Err(error) = writer.finalize_and_compress() {
                tracing::warn!(%error, "traffic log compression failed during stop");
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        self.last_message = "Traffic capture stopped; logs finalized".to_owned();
        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }

    pub fn record_snapshot(&mut self, rows: &[ProcessPortRow]) -> Result<(), TrafficLogError> {
        if !self.running {
            return Ok(());
        }

        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        for row in rows {
            for binding in &row.ports {
                let event = TrafficEvent::from_binding(&timestamp, row, binding);
                let key = LogKey {
                    pid: row.pid,
                    process_name: row.name.clone(),
                };
                let writer = match self.writers.get_mut(&key) {
                    Some(writer) if writer.bytes_written < MAX_RAW_LOG_BYTES => writer,
                    Some(_) => {
                        let old = self.writers.remove(&key).expect("writer existed");
                        old.finalize_and_compress()?;
                        self.writers
                            .entry(key.clone())
                            .or_insert(ProcessLogWriter::create(&self.root, &key)?)
                    }
                    None => self
                        .writers
                        .entry(key.clone())
                        .or_insert(ProcessLogWriter::create(&self.root, &key)?),
                };
                writer.write_event(&event)?;
            }
        }
        Ok(())
    }
}

impl Drop for TrafficCaptureManager {
    fn drop(&mut self) {
        let writers = std::mem::take(&mut self.writers);
        for (_, writer) in writers {
            if let Err(error) = writer.finalize_and_compress() {
                tracing::warn!(%error, "traffic log compression failed during shutdown");
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LogKey {
    pid: u32,
    process_name: String,
}

struct ProcessLogWriter {
    raw_path: PathBuf,
    writer: BufWriter<File>,
    bytes_written: u64,
}

impl ProcessLogWriter {
    fn create(root: &Path, key: &LogKey) -> Result<Self, TrafficLogError> {
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let folder = root.join(sanitize_path_part(&key.process_name)).join(&date);
        fs::create_dir_all(&folder).map_err(|source| TrafficLogError::CreateFolder {
            path: folder.display().to_string(),
            source,
        })?;

        let started = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let filename = format!(
            "{}_PID-{}_{}.active.jsonl",
            sanitize_path_part(&key.process_name),
            key.pid,
            started
        );
        let raw_path = folder.join(filename);
        let file = File::create(&raw_path).map_err(|source| TrafficLogError::CreateFile {
            path: raw_path.display().to_string(),
            source,
        })?;

        Ok(Self {
            raw_path,
            writer: BufWriter::new(file),
            bytes_written: 0,
        })
    }

    fn write_event(&mut self, event: &TrafficEvent<'_>) -> Result<(), TrafficLogError> {
        let mut line = serde_json::to_vec(event).map_err(|source| TrafficLogError::Serialize {
            pid: event.pid,
            source,
        })?;
        line.push(b'\n');
        self.writer
            .write_all(&line)
            .map_err(|source| TrafficLogError::Write {
                path: self.raw_path.display().to_string(),
                source,
            })?;
        self.bytes_written += line.len() as u64;
        Ok(())
    }

    fn finalize_and_compress(mut self) -> Result<(), TrafficLogError> {
        self.writer
            .flush()
            .map_err(|source| TrafficLogError::Finalize {
                path: self.raw_path.display().to_string(),
                source,
            })?;
        drop(self.writer);
        compress_jsonl_to_zstd(&self.raw_path)
    }
}

#[derive(Serialize)]
struct TrafficEvent<'a> {
    timestamp: &'a str,
    process_name: &'a str,
    pid: u32,
    protocol: &'static str,
    source_ip: String,
    source_port: u16,
    destination_ip: Option<String>,
    destination_port: Option<u16>,
    direction: &'static str,
    connection_state: Option<String>,
    event_type: &'static str,
}

impl<'a> TrafficEvent<'a> {
    fn from_binding(timestamp: &'a str, row: &'a ProcessPortRow, binding: &PortBinding) -> Self {
        let destination_port = binding.remote_port.filter(|port| *port != 0);
        let destination_ip = binding
            .remote_addr
            .filter(|addr| !is_unspecified(*addr) && destination_port.is_some())
            .map(|addr| addr.to_string());
        let connection_state = (binding.state != TcpState::NotApplicable)
            .then(|| binding.state.to_string())
            .filter(|state| !state.is_empty());
        Self {
            timestamp,
            process_name: &row.name,
            pid: row.pid,
            protocol: match binding.protocol {
                Protocol::Tcp => "TCP",
                Protocol::Udp => "UDP",
            },
            source_ip: binding.local_addr.to_string(),
            source_port: binding.local_port,
            destination_ip,
            destination_port,
            direction: direction_for(binding),
            connection_state,
            event_type: match binding.protocol {
                Protocol::Tcp => "connection_seen",
                Protocol::Udp => "endpoint_seen",
            },
        }
    }
}

pub fn open_traffic_logs(
    root: PathBuf,
    pid: u32,
    process_name: Option<String>,
) -> TrafficLogPreview {
    let mut preview = TrafficLogPreview {
        pid,
        process_name,
        root: root.clone(),
        files: Vec::new(),
        entries: Vec::new(),
        skipped_lines: 0,
        errors: Vec::new(),
    };

    let mut files = Vec::new();
    if let Err(error) = collect_matching_logs(&root, pid, &mut files) {
        preview.errors.push(format!(
            "Could not read traffic log folder {}: {error}",
            root.display()
        ));
        return preview;
    }
    files.sort();
    preview.files = files;

    let mut entries = VecDeque::new();
    for path in &preview.files {
        if let Err(error) = read_jsonl_file(path, &mut entries, &mut preview.skipped_lines) {
            preview.errors.push(format!("{}: {error}", path.display()));
        }
    }
    preview.entries = entries.into_iter().collect();
    preview
}

fn read_jsonl_file(
    path: &Path,
    entries: &mut VecDeque<String>,
    skipped_lines: &mut usize,
) -> io::Result<()> {
    let file = File::open(path)?;
    if path.extension().and_then(|value| value.to_str()) == Some("zst") {
        let decoder = zstd::stream::read::Decoder::new(file)?;
        read_jsonl_lines(BufReader::new(decoder), entries, skipped_lines)
    } else {
        read_jsonl_lines(BufReader::new(file), entries, skipped_lines)
    }
}

fn read_jsonl_lines(
    mut reader: impl BufRead,
    entries: &mut VecDeque<String>,
    skipped_lines: &mut usize,
) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }
        match serde_json::from_str::<Value>(line.trim_end()) {
            Ok(value) => {
                if entries.len() == MAX_PREVIEW_ENTRIES {
                    entries.pop_front();
                }
                entries.push_back(value.to_string());
            }
            Err(error) => {
                *skipped_lines += 1;
                tracing::warn!(%error, "skipping malformed JSONL traffic log line");
            }
        }
    }
    Ok(())
}

fn compress_jsonl_to_zstd(raw_path: &Path) -> Result<(), TrafficLogError> {
    if !raw_path.exists() {
        return Ok(());
    }

    let final_path = raw_path.with_file_name(
        raw_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.replace(".active.jsonl", ".jsonl.zst"))
            .unwrap_or_else(|| "traffic-log.jsonl.zst".to_owned()),
    );
    let tmp_path = final_path.with_extension("jsonl.zst.tmp");

    let input = File::open(raw_path).map_err(|source| TrafficLogError::Compress {
        path: raw_path.display().to_string(),
        source,
    })?;
    let output = File::create(&tmp_path).map_err(|source| TrafficLogError::Compress {
        path: tmp_path.display().to_string(),
        source,
    })?;
    let mut reader = BufReader::new(input);
    let writer = BufWriter::new(output);
    let mut encoder = zstd::stream::write::Encoder::new(writer, 3).map_err(|source| {
        TrafficLogError::Compress {
            path: tmp_path.display().to_string(),
            source,
        }
    })?;
    io::copy(&mut reader, &mut encoder).map_err(|source| TrafficLogError::Compress {
        path: raw_path.display().to_string(),
        source,
    })?;
    encoder
        .finish()
        .map_err(|source| TrafficLogError::Compress {
            path: tmp_path.display().to_string(),
            source,
        })?;

    verify_zstd_readable(&tmp_path).map_err(|source| TrafficLogError::Compress {
        path: tmp_path.display().to_string(),
        source,
    })?;
    fs::rename(&tmp_path, &final_path).map_err(|source| TrafficLogError::Compress {
        path: final_path.display().to_string(),
        source,
    })?;
    fs::remove_file(raw_path).map_err(|source| TrafficLogError::Compress {
        path: raw_path.display().to_string(),
        source,
    })?;
    Ok(())
}

fn verify_zstd_readable(path: &Path) -> io::Result<()> {
    let file = File::open(path)?;
    let mut decoder = zstd::stream::read::Decoder::new(file)?;
    let mut buffer = [0_u8; 1];
    let _ = decoder.read(&mut buffer)?;
    Ok(())
}

fn collect_matching_logs(root: &Path, pid: u32, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_matching_logs(&path, pid, files)?;
        } else if is_log_for_pid(&path, pid) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_log_for_pid(path: &Path, pid: u32) -> bool {
    let Some(filename) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let lower = filename.to_ascii_lowercase();
    lower.contains(&format!("_pid-{pid}_"))
        && (lower.ends_with(".jsonl") || lower.ends_with(".jsonl.zst"))
}

fn default_log_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("DetailedTM")
        .join("traffic-logs")
}

fn sanitize_path_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    sanitized.trim().trim_matches('.').to_owned()
}

fn direction_for(binding: &PortBinding) -> &'static str {
    match binding.protocol {
        Protocol::Tcp if binding.state == TcpState::Listening => "listening",
        Protocol::Tcp if binding.remote_port.unwrap_or_default() != 0 => "outbound",
        Protocol::Tcp => "local",
        Protocol::Udp => "local",
    }
}

fn is_unspecified(addr: Ipv4Addr) -> bool {
    addr.octets() == [0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::sanitize_path_part;

    #[test]
    fn sanitizes_windows_path_characters() {
        assert_eq!(sanitize_path_part("bad:name?.exe"), "bad_name_.exe");
    }
}
