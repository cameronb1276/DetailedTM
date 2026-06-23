use std::time::{Duration, Instant};

use crate::backend::{BackendCollector, ProcessPortRow};

pub struct AppState {
    collector: BackendCollector,
    pub rows: Vec<ProcessPortRow>,
    pub search: String,
    pub last_error: Option<String>,
    last_refresh: Option<Instant>,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            collector: BackendCollector::new(),
            rows: Vec::new(),
            search: String::new(),
            last_error: None,
            last_refresh: None,
        };
        state.refresh();
        state
    }

    pub fn refresh(&mut self) {
        match self.collector.refresh() {
            Ok(rows) => {
                self.rows = rows;
                self.last_error = None;
            }
            Err(error) => {
                tracing::error!(%error, "backend refresh failed");
                self.last_error = Some(error.to_string());
            }
        }
        self.last_refresh = Some(Instant::now());
    }

    pub fn refresh_if_due(&mut self) {
        let due = self
            .last_refresh
            .is_none_or(|last| last.elapsed() >= Duration::from_secs(1));
        if due {
            self.refresh();
        }
    }

    pub fn visible_rows(&self) -> Vec<&ProcessPortRow> {
        let needle = self.search.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|row| {
                needle.is_empty()
                    || row.name.to_lowercase().contains(&needle)
                    || row.pid.to_string().contains(&needle)
                    || row.ports_display().to_lowercase().contains(&needle)
            })
            .collect()
    }
}
