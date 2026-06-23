use std::collections::HashMap;

/// Returns per-process GPU utilization when a collector is available.
///
/// Phase 1 deliberately returns `None` for every PID. This keeps unavailable
/// data distinct from a genuine 0% reading and avoids inventing GPU values.
pub fn collect(pids: &[u32]) -> HashMap<u32, Option<f32>> {
    pids.iter().copied().map(|pid| (pid, None)).collect()
}
