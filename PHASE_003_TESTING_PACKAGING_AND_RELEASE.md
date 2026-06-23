# PHASE_003_TESTING_PACKAGING_AND_RELEASE.md

## Project
DetailedTM is a Windows 11 Rust desktop `.exe` that shows running PIDs, executable names, ports, RAM usage, CPU usage, and GPU usage.

## Phase goal
Harden the app, test it, package it, and make sure the repo is clean.

This phase must produce a release-ready Windows `.exe`.

## Required final executable
Build target:
```text
target\release\detailed_tm.exe
```

If the crate name produces a different executable name, either:
- rename the crate to produce `detailed_tm.exe`
- or document the actual executable name clearly in README

Preferred final executable:
```text
DetailedTM.exe
```

If renaming is done after build, document that process.

## Windows target
Target OS:
- Windows 11

Architecture:
- x86_64 Windows

Expected Rust target:
```text
x86_64-pc-windows-msvc
```

Use the MSVC Rust toolchain unless there is a strong reason not to.

## Build commands
Run the normal checks:

```text
cargo fmt
cargo clippy
cargo test
cargo build --release
```

If `cargo clippy` reports warnings:
- fix them when reasonable
- document any intentional remaining warnings

Do not ignore serious warnings.

## Manual test plan
Create a `TESTING.md` file.

Include manual tests for:

1. App launch
2. Process list appears
3. PID column displays valid numbers
4. name.extension column displays executable names
5. Port column shows known listening services
6. RAM column updates
7. CPU column updates
8. GPU column shows values or honest `N/A`
9. Search by Name works
10. Search by PID works
11. Search by PORT works
12. Search by extension works
13. Row selection works
14. End Task button disables when no row selected
15. End Task confirmation appears
16. Cancel does not kill process
17. End Task works on a safe test process
18. Protected process kill is blocked or fails safely
19. App does not freeze over 5 minutes
20. App closes cleanly

## Safe End Task test
Use a harmless process for End Task testing.

Examples:
- Notepad
- Calculator
- a temporary test program

Do not test kill behavior on:
- Windows system processes
- security software
- Explorer unless explicitly needed and understood
- the DetailedTM app itself

## Known ports test
Document how to verify ports with Windows built-in tools.

Use Windows commands only for verification.

Expected comparison tools:
- Task Manager
- Resource Monitor
- netstat
- PowerShell networking commands

Do not require third-party tools for basic verification.

## GPU test notes
GPU usage can be difficult to verify.

Document:
- whether GPU counters are available
- whether `GPU Engine` counters are detected
- whether per-PID matching works
- what happens when counters are unavailable

The app must not crash if GPU counters are missing.

## Logging
Add basic logging if not already present.

Logs should help diagnose:
- port collection failures
- process refresh failures
- GPU counter failures
- kill failures

Do not log sensitive user data unnecessarily.

Do not create huge log files.

For first release, console/debug logging is acceptable.

If file logging is added:
- place logs under a reasonable app data directory
- document where logs are stored

## Error handling review
Review all `unwrap`, `expect`, and panic-prone code.

Rules:
- `unwrap` is acceptable in tests.
- `unwrap` is discouraged in runtime Windows API handling.
- Runtime backend failures should return errors or partial data.
- UI should show friendly status messages.

## UI polish
Make the app usable without extra explanation.

Required polish:
- clear title
- clear search field
- clear search mode selector
- clear End Task button
- readable column widths
- visible selected row
- readable status bar

Nice-to-have:
- monospace PID/port columns
- right-aligned numeric columns
- row hover details
- copy selected row details

Do not overbuild.

## README final update
Update README with:

- Project overview
- Screenshot placeholder
- Features
- Requirements
- Build instructions
- Run instructions
- Release build instructions
- Permission notes
- GPU limitation notes
- End Task safety notes
- Troubleshooting section

## Troubleshooting section
Include common issues:

### App does not show some process names
Explain that some process details may require administrator privileges.

### App does not show GPU usage
Explain that Windows GPU Engine counters may not be available or may not expose per-PID data on all systems.

### End Task fails
Explain that Windows may deny termination for protected or elevated processes.

### Ports do not show for every process
Explain that only network-owning processes have ports.

## Versioning
Add an initial version in README.

Recommended:
```text
Version: 0.1.0
```

If using Cargo package version, keep it consistent with README.

## Optional app icon
If time allows, add a simple app icon.

Do not spend much time on icon design in this phase.

Do not use copyrighted Microsoft icons.

If no icon is added, document that it is pending.

## Release folder
Create a release output folder:

```text
release/
```

Place final build artifact or instructions there.

Recommended:
```text
release/
  README_RELEASE.txt
```

Do not commit large unnecessary build folders.

Do not commit `target/`.

If committing the `.exe`, confirm repo expectations first.

Default: do not commit compiled `.exe` unless the user requested binaries in repo.

## Gitignore review
Ensure `.gitignore` excludes:
```text
/target/
*.pdb
*.log
.DS_Store
Thumbs.db
```

Do not exclude source files.

## Security and safety review
This is a local admin utility.

It must not:
- hide from Task Manager
- auto-start without user consent
- kill processes automatically
- send process data over the network
- phone home
- collect browser history
- collect keystrokes
- collect private files

It should only inspect local process and network-port metadata.

## Final acceptance checklist
Phase 3 is complete only when:
- `cargo fmt` passes.
- `cargo clippy` passes or documented.
- `cargo test` passes.
- `cargo build --release` passes.
- Final `.exe` exists.
- README is complete.
- TESTING.md exists.
- Manual tests are documented.
- Search features still work.
- End Task confirmation still works.
- App handles GPU unavailable state.
- App does not crash during normal refresh.
- Git status is clean after commit.

## Phase 3 commit and push
After acceptance checklist passes, commit with:

```text
git add .
git commit -m "Phase 3 finalize testing packaging and release docs"
git push
```

## Final message Codex should provide
After pushing Phase 3, Codex should report:

- final repo branch
- final commit hash
- final `.exe` path
- which features passed testing
- any known limitations
- whether GPU usage is real or `N/A` fallback
- whether End Task requires admin for some processes

Do not claim success unless the commands and checks actually passed.
