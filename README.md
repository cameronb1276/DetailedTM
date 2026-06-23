# DetailedTM

DetailedTM is a native Rust desktop process monitor for Windows 11. It is being
built to show each running process, its PID, executable name, network ports, RAM,
CPU, GPU usage, and status in one useful table.

The application uses `eframe`/`egui` for its native desktop interface. Its
backend reads process metrics with `sysinfo` and associates IPv4 TCP and UDP
endpoints with owning processes through the Windows IP Helper API. It does not
run an HTTP server and does not use a web runtime.

## Build and run

Install the stable Rust toolchain on Windows 11, then run:

```powershell
cargo build
cargo run
```

For an optimized executable:

```powershell
cargo build --release
```

The executable will be written to `target\release\detailed-tm.exe`.

## Current status

Phase 1 is complete: the project foundation and internal backend collectors are
present. Process PID, executable name, status, RAM, CPU, IPv4 TCP ports, and IPv4
UDP ports are normalized into a central row model. Port-owning processes sort
first, followed by CPU usage and process name.

## Known limitations

- GPU metrics intentionally return no value in Phase 1. Per-process GPU counters
  will be implemented in the dedicated GPU phase; CPU values are never reused or
  guessed as GPU values.
- IPv6 TCP and UDP ownership collection is not yet implemented.
- The Phase 1 interface is deliberately minimal. Full search, sorting, refresh
  controls, and End Task interaction are reserved for later phases.
- Some protected Windows processes expose limited metadata without elevation.

