use std::cmp::Ordering;
use sysinfo::System;

use super::gpu_metrics;
use super::model::ProcessPortRow;
use super::{ports, process_metrics};

pub struct BackendCollector {
    system: System,
}

impl BackendCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn refresh(&mut self) -> anyhow::Result<Vec<ProcessPortRow>> {
        let mut rows = process_metrics::refresh(&mut self.system);

        let ports_by_pid = ports::collect_by_pid();

        let pids: Vec<u32> = rows.iter().map(|row| row.pid).collect();
        let gpu_by_pid = gpu_metrics::collect(&pids);

        for row in &mut rows {
            row.ports = ports_by_pid.get(&row.pid).cloned().unwrap_or_default();
            row.gpu_usage_percent = gpu_by_pid.get(&row.pid).copied().flatten();
        }

        sort_default(&mut rows);
        Ok(rows)
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
