# PHASE_002_UI_AND_TASK_MANAGER_REFRESH.md

## Project
DetailedTM is a Windows 11 Rust desktop process and port viewer.

This phase builds the user interface and connects it to the backend from Phase 1.

## Phase goal
Create a Task Manager-inspired UI that refreshes automatically and displays:

- `PID`
- `name.extension`
- `port`
- `Ram Usage`
- `CPU Usage`
- `GPU Usage`

The UI must support:
- live refresh similar to Windows Task Manager
- search by Name
- search by PID
- search by PORT
- search by extension
- selecting a row
- ending a selected task with a button and confirmation

## UI framework
Use `eframe/egui`.

Do not use:
- Electron
- Node.js
- browser frontend
- web server
- React
- Tauri unless explicitly approved later

## Window behavior
Create a native Windows desktop app.

Recommended window title:
- `DetailedTM`

Recommended starting window size:
- width: 1100
- height: 700

UI should feel similar to Windows Task Manager:
- simple top control bar
- clean table
- searchable process list
- selected row highlight
- right-side or top-right End Task button
- status footer showing refresh state and process count

Do not copy Microsoft branding, icons, or protected assets.

Use a similar practical layout, not a cloned visual identity.

## Main table columns
The table must show these exact user-facing column names:

```text
PID
name.extension
port
Ram Usage
CPU Usage
GPU Usage
```

Column behavior:
- PID: numeric
- name.extension: process executable name
- port: combined port display
- Ram Usage: readable format
- CPU Usage: percent
- GPU Usage: percent or `N/A` if unavailable

## Refresh rate
Refresh at a similar rate to Windows Task Manager.

Use a default refresh interval of about 1 second.

Make refresh interval easy to change later.

Do not refresh so aggressively that the UI freezes.

Recommended behavior:
- backend refresh every 1000 ms
- UI repaint as needed
- table state preserved between refreshes when possible

## Threading
If backend collection causes UI stutter:
- move collection to a background worker thread
- send snapshots to UI through a channel
- keep UI responsive

The UI must never freeze for several seconds during refresh.

If threading is added, document the design in README.

## Search requirements
Add a search input box.

Add a search mode selector with options:
- Name
- PID
- PORT
- extension

Search behavior:
- Case-insensitive for Name and extension.
- PID search should support exact PID and partial digits.
- PORT search should match local port text.
- Extension search should match `.exe`, `exe`, `.dll`, etc.
- Empty search shows all rows.

Recommended search examples:
- `chrome`
- `1234`
- `443`
- `.exe`

## Filtering behavior
Filtering should happen on the UI snapshot, not by re-querying Windows.

Search should be fast and not require a backend refresh.

## Sorting behavior
Allow clicking column headers to sort if practical.

Minimum sorting requirement:
- keep a stable default sort
- open-port processes should be easy to find

Preferred sorting:
- PID ascending/descending
- Name ascending/descending
- Port ascending/descending
- RAM descending
- CPU descending
- GPU descending

## Row selection
Clicking a process row should select it.

Selected row should be visually obvious.

Footer or detail area should show:
- selected PID
- selected process name
- selected ports

If selection disappears after refresh because the process ended:
- clear selection
- show a short status message

## End Task button
Add an `End Task` button.

Button behavior:
- disabled when no row is selected
- disabled when selected process is not killable
- enabled when selected process is killable

When clicked:
- show confirmation dialog
- include process name and PID in confirmation
- warn that unsaved work may be lost

Do not end the task instantly on first click.

## End Task confirmation
Confirmation dialog must have:
- `Cancel`
- `End Task`

Cancel closes dialog with no action.

End Task calls backend kill function.

After kill attempt:
- refresh process list
- show success or failure message

## Kill safety
Do not kill:
- DetailedTM itself
- PID 0
- PID 4
- protected Windows system processes when detected

If Windows denies kill:
- show error message
- do not crash

If administrator privileges are needed:
- show a clear message

## GPU usage implementation
Phase 2 should attempt real GPU usage.

Use Windows performance counters through PDH.

Task Manager-like per-process GPU usage usually comes from GPU Engine counters.

Implementation expectation:
- enumerate GPU Engine instances
- identify instances containing PID patterns
- collect `Utilization Percentage`
- sum relevant engine percentages by PID
- return a PID-to-GPU-percent map

If the GPU counter is unavailable:
- show `N/A`
- log the error
- document limitation

Do not fake GPU usage.

Do not use NVIDIA-only tooling as the only implementation.

NVIDIA-specific tools may be optional fallback only.

The primary implementation should work across Intel, AMD, and NVIDIA where Windows exposes counters.

## UI display for GPU
If GPU usage is available:
- display with one decimal place and `%`

If not available:
- display `N/A`

## RAM display
Display RAM in a readable format:
- KB for small values
- MB for normal processes
- GB for large processes

Prefer one decimal place for GB.

## CPU display
Display CPU as:
- `0.0%`
- `12.4%`

Do not show excessive decimals.

## Port display
The port column can contain:
- blank if no port
- `TCP 443 LISTENING`
- `UDP 5353`
- multiple values separated by commas

If the list is long:
- show shortened text in the table
- show full port list in row hover or detail area

## Status bar
Add a bottom status bar showing:
- number of visible processes
- number of total processes
- last refresh time
- selected PID if any
- last action message

Example:
```text
Showing 22 of 184 processes | Last refresh: 10:42:18 AM | Selected PID: 1234
```

## Error display
Non-fatal backend errors should appear in a small status area.

Do not spam modal popups every refresh.

For repeated GPU or port errors:
- show a persistent warning in status bar
- log details through tracing

## App state
Create an app state object that tracks:
- current snapshot
- filtered snapshot
- selected PID
- search text
- search mode
- sort column
- sort direction
- last refresh instant
- last status message
- pending confirmation dialog

## Performance expectations
The UI should handle hundreds of processes smoothly.

Avoid rebuilding expensive strings unnecessarily if possible.

Do not run Windows API calls directly inside every row render.

Collect first, render second.

## README update
Update README with:
- screenshot placeholder section
- UI features completed
- how refresh works
- how End Task works
- warning about permissions
- GPU usage limitations if any

## Phase 2 acceptance checklist
Phase 2 is complete only when:
- App launches as a native Windows window.
- Table shows required columns.
- Rows refresh about once per second.
- Search works by Name.
- Search works by PID.
- Search works by PORT.
- Search works by extension.
- Row selection works.
- End Task button exists.
- End Task requires confirmation.
- Kill result is shown to user.
- GPU usage is implemented or honestly displays `N/A`.
- UI does not freeze during normal refresh.
- `cargo fmt` passes.
- `cargo clippy` passes or warnings are documented.
- `cargo build` passes.

## Phase 2 commit and push
After acceptance checklist passes, commit with:

```text
git add .
git commit -m "Phase 2 add Task Manager UI search refresh and end task"
git push
```

Do not move to Phase 3 until Phase 2 has been committed and pushed.
