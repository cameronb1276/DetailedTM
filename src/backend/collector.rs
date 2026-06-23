use std::cmp::Ordering;

use sysinfo::System;

use super::gpu_metrics::GpuCollector;
use super::model::ProcessPortRow;
use super::{ports, process_metrics};

pub struct BackendRefresh {
    pub rows: Vec<ProcessPortRow>,
    pub warnings: Vec<String>,
}

pub struct BackendCollector {
    system: System,
    gpu: GpuCollector,
}

impl BackendCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            gpu: GpuCollector::new(),
        }
    }

    pub fn refresh_with_warnings(&mut self) -> BackendRefresh {
        let mut rows = process_metrics::refresh(&mut self.system);
        let (ports_by_pid, mut warnings) = ports::collect_by_pid();
        let pids: Vec<u32> = rows.iter().map(|row| row.pid).collect();
        let gpu_by_pid = match self.gpu.collect(&pids) {
            Ok(usage) => Some(usage),
            Err(error) => {
                tracing::warn!(%error, "GPU usage is unavailable");
                warnings.push(error.to_string());
                None
            }
        };

        for row in &mut rows {
            row.ports = ports_by_pid.get(&row.pid).cloned().unwrap_or_default();
            row.gpu_usage_percent = gpu_by_pid
                .as_ref()
                .and_then(|usage| usage.get(&row.pid).copied());
        }

        sort_default(&mut rows);
        BackendRefresh { rows, warnings }
    }
}

fn sort_default(rows: &mut [ProcessPortRow]) {
    rows.sort_by(|left, right| {
        left.ports
            .is_empty()
            .cmp(&right.ports.is_empty())
            .then_with(|| {
                right
                    .cpu_usage_percent
                    .partial_cmp(&left.cpu_usage_percent)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}
