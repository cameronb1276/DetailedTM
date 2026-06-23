use std::fmt;
use std::net::Ipv4Addr;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TcpState {
    Listening,
    Established,
    TimeWait,
    CloseWait,
    Unknown,
    NotApplicable,
}

impl fmt::Display for TcpState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Listening => "LISTENING",
            Self::Established => "ESTABLISHED",
            Self::TimeWait => "TIME_WAIT",
            Self::CloseWait => "CLOSE_WAIT",
            Self::Unknown => "UNKNOWN",
            Self::NotApplicable => "",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortBinding {
    pub pid: u32,
    pub protocol: Protocol,
    pub local_addr: Ipv4Addr,
    pub local_port: u16,
    pub remote_addr: Option<Ipv4Addr>,
    pub remote_port: Option<u16>,
    pub state: TcpState,
    pub tcp_state_code: Option<u32>,
    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,
    pub upload_rate_bytes_per_second: Option<f64>,
    pub download_rate_bytes_per_second: Option<f64>,
}

impl PortBinding {
    pub fn display_compact(&self) -> String {
        let state = self.state.to_string();
        if state.is_empty() {
            format!("{} {}", self.protocol, self.local_port)
        } else {
            format!("{} {} {state}", self.protocol, self.local_port)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessPortRow {
    pub pid: u32,
    pub name: String,
    pub extension: String,
    pub ports: Vec<PortBinding>,
    pub ram_usage_bytes: u64,
    pub ram_usage_display: String,
    pub cpu_usage_percent: f32,
    pub gpu_usage_percent: Option<f32>,
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_rate_bytes_per_second: f64,
    pub download_rate_bytes_per_second: f64,
    pub network_usage_available: bool,
    pub is_killable: bool,
    pub status: String,
    pub last_seen: Instant,
}

impl ProcessPortRow {
    pub fn ports_display(&self) -> String {
        self.ports
            .iter()
            .map(PortBinding::display_compact)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
