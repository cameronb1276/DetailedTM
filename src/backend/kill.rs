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

pub fn is_killable(pid: u32, process_name: &str, current_pid: u32) -> bool {
    pid != 0
        && pid != 4
        && pid != current_pid
        && !CRITICAL_PROCESS_NAMES
            .iter()
            .any(|critical| process_name.eq_ignore_ascii_case(critical))
}
