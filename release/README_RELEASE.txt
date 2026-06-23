DetailedTM 0.1.0 - Windows 11 x86_64 Release
================================================

Build date: 2026-06-22
Toolchain: stable-x86_64-pc-windows-msvc (rustc 1.96.0)
Source package version: 0.1.0

Preferred local artifact
------------------------

Build command:

  cargo build --release

Artifact:

  target\release\DetailedTM.exe

Size: 6,330,880 bytes
SHA-256: 3E4ADE21A024741E66B92B78049E4834ACA6A7AFFCDD9B531A1BBF260ACC1339

Explicit-target artifact
------------------------

Build command:

  cargo build --release --target x86_64-pc-windows-msvc

Artifact:

  target\x86_64-pc-windows-msvc\release\DetailedTM.exe

Size: 6,332,416 bytes
SHA-256: 5CA1F3D2C6DDD2ABA32325D2265903AD23A17160B22198C9B0AED17247B52644

Compiled executables are intentionally excluded from Git. Rebuild locally from
the committed source and verify the newly produced artifact for distribution.

Release verification
--------------------

PASS  cargo fmt -- --check
PASS  cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
PASS  cargo test --target x86_64-pc-windows-msvc (5 passed, 0 failed)
PASS  cargo build --release --target x86_64-pc-windows-msvc
PASS  cargo build --release
PASS  Native window launch and required-column UI Automation inspection
PASS  Name/PID/PORT/extension search unit coverage
PASS  Live IPv4 TCP and UDP ownership mapping to the current test PID
PASS  Row selection and End Task enabled state for a controlled process
PASS  End Task confirmation appeared for a controlled PowerShell sleep process
PASS  Cancel preserved the controlled process
PASS  Confirmed End Task stopped the controlled process
PASS  Selecting protected PID 4 kept End Task disabled
PASS  Five-minute soak: ten 30-second samples advanced the refresh timestamp,
      retained a responsive UI Automation tree, and closed cleanly

GPU verification
----------------

Windows GPU Engine counters were available on the release test machine:

  492 Utilization Percentage samples
  492 PID-tagged instances

DetailedTM displayed real PDH-derived percentages, including valid 0.0% values.
On machines without these counters it displays N/A and a non-fatal status warning.

Distribution notes
------------------

This first release is not code-signed and has no custom icon. Windows may show an
unrecognized-app warning when the executable is obtained from another computer.
See README.md for requirements, permissions, limitations, and troubleshooting;
see TESTING.md for the complete repeatable test plan.
