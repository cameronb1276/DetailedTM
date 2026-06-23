# DetailedTM 0.1.0 Testing

Target environment: Windows 11 x86_64 using the stable
`x86_64-pc-windows-msvc` Rust toolchain.

## Automated release gates

Run from the repository root:

```powershell
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --target x86_64-pc-windows-msvc
```

The test suite covers memory formatting, live IPv4 TCP and UDP ownership mapping
to the current PID, all four search modes, GPU Engine PID parsing, and protected
process classification. The release window is also inspected through Windows UI
Automation to verify its title and required controls.

## Manual test plan

Use a normal user session first. Repeat permission-sensitive checks from an
elevated session only if needed. Never attempt termination of a system, security,
Explorer, or DetailedTM process.

| # | Test | Procedure | Expected result |
|---:|---|---|---|
| 1 | App launch | Start `DetailedTM.exe`. | A native window titled `DetailedTM` opens. |
| 2 | Process list | Wait for the first refresh. | Running processes appear within about one second. |
| 3 | PID column | Inspect several rows. | PID values are numeric and agree with Task Manager. |
| 4 | name.extension | Inspect known apps. | Executable names include extensions where Windows exposes them. |
| 5 | Known port | Start a known listener and compare with `netstat -ano`. | Matching TCP/UDP local port appears on the owning PID. |
| 6 | RAM updates | Exercise a user app and watch two refreshes. | Readable KB/MB/GB values update without flicker. |
| 7 | CPU updates | Give a user app brief CPU work. | CPU percentage updates with one decimal place. |
| 8 | GPU behavior | Run `Get-Counter '\GPU Engine(*)\Utilization Percentage'`, then exercise a GPU app. | Values appear when per-PID counters exist; otherwise honest `N/A` and a warning appear. |
| 9 | Name search | Select Name and enter part of a known executable. | Matching names remain, case-insensitively. |
| 10 | PID search | Select PID and enter part of a PID. | Rows containing those PID digits remain. |
| 11 | PORT search | Select PORT and enter part of a known local port. | Matching endpoint owners remain. |
| 12 | Extension search | Select extension and enter `.exe`, then `exe`. | Both forms match executable extensions. |
| 13 | Row selection | Click a process row. | Row highlight and selected name, PID, and ports are visible. |
| 14 | No-selection safety | Launch without selecting a row. | End Task is disabled. |
| 15 | Confirmation | Select a harmless process and click End Task. | Confirmation names the process and PID and warns about unsaved work. |
| 16 | Cancel safety | Click Cancel in that confirmation. | Dialog closes and the process remains running. |
| 17 | Safe End Task | Start Notepad, select its exact row, confirm End Task. | Notepad exits and a success message appears. |
| 18 | Protected process | Select PID 4/System if visible. | End Task remains disabled; no termination is attempted. |
| 19 | Five-minute stability | Leave the app open and interact with search/sort periodically for five minutes. | Refresh continues near one second and the UI remains responsive. |
| 20 | Clean close | Close the window normally. | Window and background collector exit cleanly. |

## Built-in port comparison

No third-party utility is required:

```powershell
netstat -ano
Get-NetTCPConnection | Sort-Object OwningProcess,LocalPort
Get-NetUDPEndpoint | Sort-Object OwningProcess,LocalPort
```

For a controlled listener, PowerShell can create a temporary local socket; close
it after verifying that its local port and PowerShell PID appear together.

## GPU notes

The primary collector is Windows PDH, not vendor-specific tooling. Availability
can be checked with:

```powershell
Get-Counter '\GPU Engine(*)\Utilization Percentage'
```

DetailedTM accepts both valid and newly collected PDH samples, extracts `_pid_N_`
from engine instance names, sums engines for each PID, and caps the UI percentage
at 100. Missing counters, invalid first samples, or drivers without per-process
data produce `N/A` plus a non-fatal warning; the app must continue refreshing.

## Release verification record

The final commands, artifact hash, native-window smoke test, UI Automation check,
safe End Task exercise, and five-minute stability result are recorded in
`release\README_RELEASE.txt` when the release is built.

