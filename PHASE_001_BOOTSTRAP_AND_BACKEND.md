# PHASE_001_BOOTSTRAP_AND_BACKEND.md

## Project
Build `DetailedTM`, a Windows 11 Rust desktop program that lists running processes, their PID, executable name, ports, RAM usage, CPU usage, and GPU usage.

## Hard requirements
- Work directory must be:
  - `C:\sloansites backup\DetailedTM`
- GitHub repo must be:
  - `git@github.com:cameronb1276/DetailedTM.git`
- Git commits must use only:
  - Username: `cameronb1276`
  - Email: `cameron9823@gmail.com`
- Language must be Rust.
- Final app must compile to a `.exe` for Windows 11.
- Do not make this a web app.
- Do not use Electron.
- Do not use Node.js.
- Do not use Tauri unless explicitly approved later.
- Prefer a pure Rust desktop GUI using `eframe/egui`.

## Phase goal
Create the project foundation and backend data collection layer.

The backend must collect:
- PID
- process executable name, including extension when available
- port or ports owned by that PID
- RAM usage
- CPU usage
- GPU usage placeholder or real collector interface
- process status metadata needed by the UI

## Important note about "backend"
For this project, "backend" means internal Rust modules that collect and normalize Windows system data.

Do not create an HTTP API server unless specifically instructed later.

## Expected workspace structure
Inside `C:\sloansites backup\DetailedTM`, create or normalize this structure:

```text
DetailedTM/
  Cargo.toml
  README.md
  .gitignore
  src/
    main.rs
    app/
      mod.rs
      state.rs
    backend/
      mod.rs
      collector.rs
      model.rs
      ports.rs
      process_metrics.rs
      gpu_metrics.rs
      kill.rs
    ui/
      mod.rs
      table.rs
      search.rs
      controls.rs
    tests/
      mod.rs
```

## Cargo dependencies
Use stable, well-maintained crates.

Recommended dependencies:
- `eframe` for native egui desktop app
- `egui` through eframe
- `egui_extras` if a better table widget is needed
- `sysinfo` for process list, RAM, CPU, and process metadata
- `windows-sys` or `windows` for Windows-specific APIs
- `anyhow` for application-level errors
- `thiserror` for typed backend errors
- `serde` only if needed for future config/export
- `tracing` and `tracing-subscriber` for debug logging

Do not add unnecessary dependencies.

## Rust edition
Use Rust 2021 or newer.

If Rust 2024 is supported locally, it is acceptable, but avoid unstable features.

## Backend model
Create a central row model named something close to:

`ProcessPortRow`

It must represent one displayed row in the table.

Fields required:
- `pid`
- `name`
- `extension`
- `ports`
- `ram_usage_bytes`
- `ram_usage_display`
- `cpu_usage_percent`
- `gpu_usage_percent`
- `is_killable`
- `last_seen`

The `ports` field can be a list internally.

The UI display can join ports into one string.

## Port model
Create a port model named something close to:

`PortBinding`

Fields required:
- `pid`
- `protocol`
- `local_addr`
- `local_port`
- `remote_addr`
- `remote_port`
- `state`

Protocol should support:
- TCP
- UDP

State should support:
- LISTENING
- ESTABLISHED
- TIME_WAIT
- CLOSE_WAIT
- UNKNOWN
- blank or not applicable for UDP

## Windows port collection
Implement Windows port-to-PID collection.

Use Windows IP Helper APIs:
- TCP table with owning PID
- UDP table with owning PID

Minimum requirement:
- IPv4 TCP
- IPv4 UDP

Stretch requirement:
- IPv6 TCP
- IPv6 UDP

The backend must group all ports by PID.

For display:
- If a PID owns multiple ports, show comma-separated ports.
- Include enough protocol/state detail to be useful.
- Example display:
  - `TCP 443 LISTENING`
  - `TCP 5173 ESTABLISHED`
  - `UDP 5353`

## Process collection
Use `sysinfo` to refresh process data.

Collect:
- PID
- executable/process name
- RAM usage
- CPU usage

Executable name rule:
- Prefer the executable file name from the process path.
- Fall back to process name if path is unavailable.
- Keep `.exe` or other extension when available.
- If no extension exists, extension should be an empty string.

RAM display rule:
- Store raw bytes internally.
- Display in KB, MB, or GB depending on size.
- Prefer Task Manager-like readability.

CPU display rule:
- Store numeric percent internally.
- Display with one decimal place.
- Avoid fake precision.

## GPU collection interface
Create `gpu_metrics.rs` now, even if real GPU usage is implemented in Phase 2.

Define a clean function boundary:
- input: process PID list
- output: map of PID to GPU usage percent

For Phase 1, it may return `None` or `0.0` with a clear TODO.

Do not fake GPU usage.

Do not randomly guess GPU usage from CPU usage.

## Refresh behavior
Create backend refresh function with a clean API.

Recommended shape:
- refresh all process data
- refresh port data
- refresh GPU data
- merge by PID
- return sorted row list

The function should be usable by the UI once per second.

Do not block the UI thread longer than necessary.

If collection is slow, prepare the code so it can later move to a worker thread.

## Sorting default
Default backend sort should be:
1. processes with open ports first
2. then highest CPU usage
3. then process name ascending

The UI can override this later.

## Safety requirements
The app will include an End Task button later.

Prepare backend kill logic in `kill.rs`, but do not expose it to UI in this phase unless complete.

Do not allow killing:
- PID 0
- PID 4
- the DetailedTM process itself
- obvious critical Windows system processes without a confirmation path

Critical processes should be marked as not killable where possible.

## Error handling
Backend must not panic during normal collection.

If one data source fails:
- log the error
- continue with other available fields
- return partial data

Example:
- if GPU counters fail, still show PID, name, ports, RAM, and CPU.

## README requirements for Phase 1
Create `README.md` with:
- project name
- purpose
- Windows 11 target
- how to build
- how to run
- current phase status
- known limitations

Mention that GPU usage may be incomplete until the GPU phase is completed.

## Git setup
Before coding, configure local git identity only for this repo:

```text
git config user.name "cameronb1276"
git config user.email "cameron9823@gmail.com"
```

Do not use global git config.

Initialize repo if needed.

If remote does not exist, add:
```text
git remote add origin git@github.com:cameronb1276/DetailedTM.git
```

If origin exists but points somewhere else, stop and report the mismatch.

## Phase 1 acceptance checklist
Phase 1 is complete only when:
- Rust project builds successfully.
- Backend can collect a list of running PIDs.
- Backend can collect process names.
- Backend can collect RAM usage.
- Backend can collect CPU usage.
- Backend can collect TCP and UDP ports by PID.
- Backend can merge process and port data.
- GPU module exists with honest placeholder behavior if not implemented yet.
- README exists.
- `.gitignore` exists and excludes `target/`.
- `cargo fmt` passes.
- `cargo clippy` passes or warnings are documented.
- `cargo build` passes.

## Phase 1 commit and push
After acceptance checklist passes, commit with:

```text
git add .
git commit -m "Phase 1 bootstrap backend process and port collection"
git push -u origin main
```

If the branch is named `master`, rename it to `main` before pushing unless repo already requires otherwise.

Do not move to Phase 2 until Phase 1 has been committed and pushed.
