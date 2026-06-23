use thiserror::Error;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED};
use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

const CRITICAL_PROCESS_NAMES: &[&str] = &[
    "csrss.exe",
    "dwm.exe",
    "lsass.exe",
    "services.exe",
    "smss.exe",
    "svchost.exe",
    "system",
    "wininit.exe",
    "winlogon.exe",
];

#[derive(Debug, Error)]
pub enum KillError {
    #[error("DetailedTM will not terminate protected process {name} (PID {pid})")]
    Protected { pid: u32, name: String },
    #[error("Windows denied permission to end {name} (PID {pid}); administrator privileges may be required")]
    AccessDenied { pid: u32, name: String },
    #[error("Windows could not open {name} (PID {pid}); error {code}")]
    OpenFailed { pid: u32, name: String, code: u32 },
    #[error("Windows could not end {name} (PID {pid}); error {code}")]
    TerminateFailed { pid: u32, name: String, code: u32 },
}

pub fn is_killable(pid: u32, process_name: &str, current_pid: u32) -> bool {
    pid != 0
        && pid != 4
        && pid != current_pid
        && !CRITICAL_PROCESS_NAMES
            .iter()
            .any(|critical| process_name.eq_ignore_ascii_case(critical))
}

pub fn kill_process(pid: u32, process_name: &str) -> Result<(), KillError> {
    if !is_killable(pid, process_name, std::process::id()) {
        return Err(KillError::Protected {
            pid,
            name: process_name.to_owned(),
        });
    }

    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return Err(open_error(pid, process_name, unsafe { GetLastError() }));
    }

    let terminated = unsafe { TerminateProcess(handle, 1) };
    let error = if terminated == 0 {
        Some(unsafe { GetLastError() })
    } else {
        None
    };
    unsafe { CloseHandle(handle) };

    match error {
        None => Ok(()),
        Some(ERROR_ACCESS_DENIED) => Err(KillError::AccessDenied {
            pid,
            name: process_name.to_owned(),
        }),
        Some(code) => Err(KillError::TerminateFailed {
            pid,
            name: process_name.to_owned(),
            code,
        }),
    }
}

fn open_error(pid: u32, process_name: &str, code: u32) -> KillError {
    if code == ERROR_ACCESS_DENIED {
        KillError::AccessDenied {
            pid,
            name: process_name.to_owned(),
        }
    } else {
        KillError::OpenFailed {
            pid,
            name: process_name.to_owned(),
            code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_killable;

    #[test]
    fn protects_system_and_current_processes() {
        let current = std::process::id();
        assert!(!is_killable(0, "Idle", current));
        assert!(!is_killable(4, "System", current));
        assert!(!is_killable(current, "detailed-tm.exe", current));
        assert!(!is_killable(999, "lsass.exe", current));
        assert!(is_killable(999, "notepad.exe", current));
    }
}
