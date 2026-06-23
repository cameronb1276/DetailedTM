use std::collections::HashMap;
use std::ffi::c_void;
use std::net::Ipv4Addr;

use thiserror::Error;
use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::AF_INET;

use super::model::{PortBinding, Protocol, TcpState};

const TCP_ROW_DWORDS: usize = 6;
const UDP_ROW_DWORDS: usize = 3;

#[derive(Debug, Error)]
pub enum PortCollectionError {
    #[error("{table} size query failed with Windows error {code}")]
    SizeQuery { table: &'static str, code: u32 },
    #[error("{table} collection failed with Windows error {code}")]
    Collection { table: &'static str, code: u32 },
    #[error("{table} returned a truncated table")]
    Truncated { table: &'static str },
}

pub fn collect_by_pid() -> HashMap<u32, Vec<PortBinding>> {
    let mut grouped = HashMap::<u32, Vec<PortBinding>>::new();

    match collect_tcp_ipv4() {
        Ok(bindings) => {
            for binding in bindings {
                grouped.entry(binding.pid).or_default().push(binding);
            }
        }
        Err(error) => tracing::warn!(%error, "IPv4 TCP collection failed"),
    }
    match collect_udp_ipv4() {
        Ok(bindings) => {
            for binding in bindings {
                grouped.entry(binding.pid).or_default().push(binding);
            }
        }
        Err(error) => tracing::warn!(%error, "IPv4 UDP collection failed"),
    }

    for bindings in grouped.values_mut() {
        bindings.sort_by_key(|binding| (binding.protocol.to_string(), binding.local_port));
    }
    grouped
}

fn collect_tcp_ipv4() -> Result<Vec<PortBinding>, PortCollectionError> {
    let words = query_table("TCP", |table, size| unsafe {
        GetExtendedTcpTable(table, size, 0, AF_INET as u32, TCP_TABLE_OWNER_PID_ALL, 0)
    })?;
    parse_rows("TCP", &words, TCP_ROW_DWORDS, |row| PortBinding {
        pid: row[5],
        protocol: Protocol::Tcp,
        local_addr: ipv4(row[1]),
        local_port: network_port(row[2]),
        remote_addr: Some(ipv4(row[3])),
        remote_port: Some(network_port(row[4])),
        state: tcp_state(row[0]),
    })
}

fn collect_udp_ipv4() -> Result<Vec<PortBinding>, PortCollectionError> {
    let words = query_table("UDP", |table, size| unsafe {
        GetExtendedUdpTable(table, size, 0, AF_INET as u32, UDP_TABLE_OWNER_PID, 0)
    })?;
    parse_rows("UDP", &words, UDP_ROW_DWORDS, |row| PortBinding {
        pid: row[2],
        protocol: Protocol::Udp,
        local_addr: ipv4(row[0]),
        local_port: network_port(row[1]),
        remote_addr: None,
        remote_port: None,
        state: TcpState::NotApplicable,
    })
}

fn query_table(
    table_name: &'static str,
    query: impl Fn(*mut c_void, *mut u32) -> u32,
) -> Result<Vec<u32>, PortCollectionError> {
    let mut byte_size = 0_u32;
    let initial = query(std::ptr::null_mut(), &mut byte_size);
    if initial != ERROR_INSUFFICIENT_BUFFER && initial != NO_ERROR {
        return Err(PortCollectionError::SizeQuery {
            table: table_name,
            code: initial,
        });
    }

    let word_count = (byte_size as usize).div_ceil(std::mem::size_of::<u32>());
    let mut words = vec![0_u32; word_count.max(1)];
    let result = query(words.as_mut_ptr().cast(), &mut byte_size);
    if result != NO_ERROR {
        return Err(PortCollectionError::Collection {
            table: table_name,
            code: result,
        });
    }
    Ok(words)
}

fn parse_rows(
    table_name: &'static str,
    words: &[u32],
    row_width: usize,
    convert: impl Fn(&[u32]) -> PortBinding,
) -> Result<Vec<PortBinding>, PortCollectionError> {
    let Some((&count, rows)) = words.split_first() else {
        return Err(PortCollectionError::Truncated { table: table_name });
    };
    let required = count as usize * row_width;
    if rows.len() < required {
        return Err(PortCollectionError::Truncated { table: table_name });
    }
    Ok(rows[..required]
        .chunks_exact(row_width)
        .map(convert)
        .collect())
}

fn ipv4(raw: u32) -> Ipv4Addr {
    Ipv4Addr::from(raw.to_ne_bytes())
}

fn network_port(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

fn tcp_state(raw: u32) -> TcpState {
    match raw {
        2 => TcpState::Listening,
        5 => TcpState::Established,
        8 => TcpState::CloseWait,
        11 => TcpState::TimeWait,
        _ => TcpState::Unknown,
    }
}
