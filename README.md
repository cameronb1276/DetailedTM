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

Phase 2 is complete. The Task Manager-inspired native interface provides the six
required columns, clickable sorting, row selection, a selected-process detail
line, and filtering by name, partial PID, local port, or extension. The status
bar reports visible and total process counts, refresh time, selected PID, action
results, and non-fatal collector warnings.

Snapshots are collected by a dedicated Rust worker thread about once per second
and delivered to egui over a bounded channel. Search and sorting operate only on
the latest in-memory snapshot, so typing and table interaction never query
Windows or wait for a refresh.

## End Task

Select a row and press **End Task**. DetailedTM shows a confirmation containing
the executable name and PID before calling the Windows process API. PID 0, PID 4,
DetailedTM itself, and recognized critical Windows processes are disabled. Other
protected processes may still be rejected by Windows; the status bar reports the
failure and explains when administrator privileges may be required. Ending a
process can discard unsaved work.

## Screenshot

_Screenshot placeholder — a release screenshot will be added during packaging._

## Known limitations

- GPU usage is sampled from Windows' cross-vendor `\GPU Engine(*)\Utilization
  Percentage` PDH counter and aggregated by the PID embedded in each engine
  instance. If Windows or a display driver does not expose that counter, the UI
  honestly displays `N/A` and keeps a persistent warning in the status bar. CPU
  values are never reused or guessed as GPU values.
- IPv6 TCP and UDP ownership collection is not yet implemented.
- Some protected Windows processes expose limited metadata without elevation.
