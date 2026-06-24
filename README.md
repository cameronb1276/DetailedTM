# DetailedTM

**Version: 0.1.0**

DetailedTM is a native Windows 11 process and network-port viewer written in
Rust. It combines Task Manager-style CPU, memory, and GPU readings with the IPv4
TCP and UDP endpoints owned by each process. DetailedTM runs locally, has no web
runtime or server, sends no telemetry, and never ends a process automatically.

## Features

- Native `eframe`/`egui` Windows desktop interface
- PID, executable name, owned ports, RAM, CPU, and GPU columns
- One-second snapshots collected on a background thread to keep the UI responsive
- Case-insensitive name and extension search
- Partial PID and local-port search
- Clickable sorting, selected-row highlighting, and full port details
- Confirmation-gated End Task with critical-process safeguards
- Partial-data behavior and visible warnings when a Windows collector is unavailable

## Screenshot

![DetailedTM process and port monitor](./DetailedTM%20screenshot.png)

## Requirements

- Windows 11, x86_64
- For building: stable Rust with the `x86_64-pc-windows-msvc` target and MSVC
  build tools
- Administrator rights are not required to view ordinary processes, but Windows
  can restrict metadata and termination of protected or elevated processes

## Build and run

From PowerShell in the repository root:

```powershell
cargo build
cargo run
```

The default build uses the native Windows target and opens a desktop window named
`DetailedTM`.

## Release build

```powershell
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --target x86_64-pc-windows-msvc
```

The release executable is:

```text
target\x86_64-pc-windows-msvc\release\DetailedTM.exe
```

When the explicit `--target` flag is omitted on an x86_64 MSVC host, Cargo writes
the same program to `target\release\DetailedTM.exe`. Release packaging notes are
in `release\README_RELEASE.txt`; compiled binaries are intentionally not committed.

## Refresh design

A dedicated Rust worker owns the process, port, and PDH collectors. It produces a
snapshot about once per second and sends it to egui over a bounded channel. Search
and sorting use only the latest in-memory snapshot, so typing and table rendering
do not invoke Windows APIs.

## GPU usage

DetailedTM uses Windows PDH's cross-vendor
`\GPU Engine(*)\Utilization Percentage` counter. It extracts PIDs from GPU engine
instances and aggregates their utilization per process. When the counter or
per-PID data is unavailable, the GPU column displays `N/A`, a warning appears in
the status bar, and the other collectors continue. CPU data is never substituted
for GPU data.

## End Task safety and permissions

End Task is disabled until a killable row is selected. A second confirmation
shows the executable name and PID and warns about unsaved work. DetailedTM blocks
PID 0, PID 4, itself, and recognized critical Windows processes. Windows may deny
termination of other protected or elevated processes unless DetailedTM is run as
administrator; that failure is reported without crashing.

## Port verification

Only processes that own network endpoints have port entries. Compare a PID and
local port using built-in Windows tools:

```powershell
netstat -ano
Get-NetTCPConnection | Select-Object LocalAddress,LocalPort,State,OwningProcess
Get-NetUDPEndpoint | Select-Object LocalAddress,LocalPort,OwningProcess
```

Task Manager and Resource Monitor can provide an additional visual comparison.

## Logging

Tracing reports collector and End Task failures to the debug console when one is
available. Set `RUST_LOG=detailed_tm=debug` before a debug run for more detail.
The first release does not create a log file, so it cannot accumulate large logs.

## Troubleshooting

### App does not show some process names

Windows limits details for some protected or elevated processes. Run DetailedTM
as administrator only when that access is necessary.

### App does not show GPU usage

The display driver or Windows installation may not expose GPU Engine counters, or
the counters may not include per-PID instances. DetailedTM shows `N/A` and keeps
the process, port, RAM, and CPU data available.

### End Task fails

Windows denies termination of protected processes and can deny access across
elevation boundaries. Select an ordinary user process or, when appropriate, run
DetailedTM as administrator. Built-in critical-process blocks remain in effect.

### Ports do not show for every process

Most processes do not own a TCP or UDP endpoint. Phase 0.1.0 collects IPv4 TCP
and UDP ownership; IPv6 endpoint collection remains a known limitation.

## Known limitations

- GPU readings depend on Windows and driver-provided PDH GPU Engine counters.
- IPv6 TCP and UDP ownership is not yet collected.
- Some protected-process metadata and termination require elevation or remain
  unavailable even when elevated.
- A custom application icon is pending.

See [TESTING.md](TESTING.md) for the release test plan and recorded results.
