use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::net::Ipv4Addr;
use std::time::Instant;

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetPerTcpConnectionEStats, SetPerTcpConnectionEStats, TCP_ESTATS_DATA_ROD_v0,
    TCP_ESTATS_DATA_RW_v0, TcpConnectionEstatsData, MIB_TCPROW_LH, MIB_TCPROW_LH_0,
};

use super::model::{PortBinding, Protocol};

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessNetworkUsage {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_rate_bytes_per_second: f64,
    pub download_rate_bytes_per_second: f64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ConnectionKey {
    pid: u32,
    local_addr: Ipv4Addr,
    local_port: u16,
    remote_addr: Ipv4Addr,
    remote_port: u16,
}

pub struct NetworkCollector {
    enabled: HashSet<ConnectionKey>,
    previous: HashMap<ConnectionKey, (u64, u64)>,
    totals: HashMap<u32, (u64, u64)>,
    last_sample: Instant,
    available: bool,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            enabled: HashSet::new(),
            previous: HashMap::new(),
            totals: HashMap::new(),
            last_sample: Instant::now(),
            available: false,
        }
    }

    pub fn collect(
        &mut self,
        ports_by_pid: &mut HashMap<u32, Vec<PortBinding>>,
        live_pids: &[u32],
    ) -> (HashMap<u32, ProcessNetworkUsage>, Option<String>, bool) {
        let elapsed = self.last_sample.elapsed().as_secs_f64().max(0.001);
        self.last_sample = Instant::now();
        let live: HashSet<u32> = live_pids.iter().copied().collect();
        self.totals.retain(|pid, _| live.contains(pid));

        let mut active = HashSet::new();
        let mut interval_by_pid = HashMap::<u32, (u64, u64)>::new();
        let mut failed = 0_usize;
        let mut first_error = None;

        for bindings in ports_by_pid.values_mut() {
            for binding in bindings {
                let Some((key, row)) = connection_row(binding) else {
                    continue;
                };
                active.insert(key);

                if !self.enabled.contains(&key) {
                    let settings = TCP_ESTATS_DATA_RW_v0 {
                        EnableCollection: true,
                    };
                    let status = unsafe {
                        SetPerTcpConnectionEStats(
                            &row,
                            TcpConnectionEstatsData,
                            (&settings as *const TCP_ESTATS_DATA_RW_v0).cast(),
                            0,
                            size_of::<TCP_ESTATS_DATA_RW_v0>() as u32,
                            0,
                        )
                    };
                    if status != NO_ERROR {
                        failed += 1;
                        first_error.get_or_insert(status);
                        continue;
                    }
                    self.enabled.insert(key);
                    self.previous.insert(key, (0, 0));
                }

                let mut data = TCP_ESTATS_DATA_ROD_v0::default();
                let status = unsafe {
                    GetPerTcpConnectionEStats(
                        &row,
                        TcpConnectionEstatsData,
                        std::ptr::null_mut(),
                        0,
                        0,
                        std::ptr::null_mut(),
                        0,
                        0,
                        (&mut data as *mut TCP_ESTATS_DATA_ROD_v0).cast(),
                        0,
                        size_of::<TCP_ESTATS_DATA_ROD_v0>() as u32,
                    )
                };
                if status != NO_ERROR {
                    failed += 1;
                    first_error.get_or_insert(status);
                    continue;
                }
                self.available = true;

                let previous = self
                    .previous
                    .insert(key, (data.DataBytesOut, data.DataBytesIn));
                let (previous_out, previous_in) = previous.unwrap_or_default();
                let upload_delta = data.DataBytesOut.saturating_sub(previous_out);
                let download_delta = data.DataBytesIn.saturating_sub(previous_in);
                let interval = interval_by_pid.entry(binding.pid).or_default();
                interval.0 = interval.0.saturating_add(upload_delta);
                interval.1 = interval.1.saturating_add(download_delta);
                let totals = self.totals.entry(binding.pid).or_default();
                totals.0 = totals.0.saturating_add(upload_delta);
                totals.1 = totals.1.saturating_add(download_delta);

                binding.bytes_sent = Some(data.DataBytesOut);
                binding.bytes_received = Some(data.DataBytesIn);
                binding.upload_rate_bytes_per_second = Some(upload_delta as f64 / elapsed);
                binding.download_rate_bytes_per_second = Some(download_delta as f64 / elapsed);
            }
        }

        self.enabled.retain(|key| active.contains(key));
        self.previous.retain(|key, _| active.contains(key));

        let usage = live_pids
            .iter()
            .copied()
            .map(|pid| {
                let totals = self.totals.get(&pid).copied().unwrap_or_default();
                let interval = interval_by_pid.get(&pid).copied().unwrap_or_default();
                (
                    pid,
                    ProcessNetworkUsage {
                        upload_bytes: totals.0,
                        download_bytes: totals.1,
                        upload_rate_bytes_per_second: interval.0 as f64 / elapsed,
                        download_rate_bytes_per_second: interval.1 as f64 / elapsed,
                    },
                )
            })
            .collect();

        let warning = first_error.map(|error| {
            if error == ERROR_ACCESS_DENIED {
                format!(
                    "TCP byte counters were denied for {failed} connection(s); run as administrator for per-process upload/download tracking"
                )
            } else {
                format!(
                    "TCP byte counters were unavailable for {failed} connection(s) (Windows error {error})"
                )
            }
        });
        (usage, warning, self.available)
    }
}

fn connection_row(binding: &PortBinding) -> Option<(ConnectionKey, MIB_TCPROW_LH)> {
    if binding.protocol != Protocol::Tcp {
        return None;
    }
    let remote_addr = binding.remote_addr?;
    let remote_port = binding.remote_port?;
    let state = binding.tcp_state_code?;
    if state != 5 || remote_port == 0 || remote_addr.is_unspecified() {
        return None;
    }

    let key = ConnectionKey {
        pid: binding.pid,
        local_addr: binding.local_addr,
        local_port: binding.local_port,
        remote_addr,
        remote_port,
    };
    let row = MIB_TCPROW_LH {
        Anonymous: MIB_TCPROW_LH_0 { dwState: state },
        dwLocalAddr: u32::from_ne_bytes(binding.local_addr.octets()),
        dwLocalPort: u16::to_be(binding.local_port) as u32,
        dwRemoteAddr: u32::from_ne_bytes(remote_addr.octets()),
        dwRemotePort: u16::to_be(remote_port) as u32,
    };
    Some((key, row))
}
