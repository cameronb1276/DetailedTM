DetailedTM 0.2.0 - Windows 11 x86_64 Release
================================================

Build date: 2026-06-22
Toolchain: stable-x86_64-pc-windows-msvc (rustc 1.96.0)
Source package version: 0.2.0

Build command:

  cargo build --release

Cargo artifact:

  target\release\DetailedTM.exe

User-facing copy:

  DetailedTM.exe

Size: 6,413,312 bytes
SHA-256: 7929EBE3B5FC31C0CF4C329991B219F615AC8D87C58967B8BD2119B8BE46C0A7

Version 0.2.0 network visibility
--------------------------------

- Adds per-process IPv4 TCP upload/download totals and rates through Windows
  TCP Extended Statistics (EStats).
- Adds local and remote IP:port destination details for selected processes.
- Requires "Run as administrator" for Windows to enable TCP byte counters.
- Shows N/A and a clear warning when those counters are denied or unavailable.
- Does not install a packet driver, intercept TLS, or capture payloads.
- HTTPS commands, files, bodies, and full URLs remain encrypted and are labeled
  unavailable instead of being inferred.
- UDP and IPv6 byte totals are not measured in this release.

Verification
------------

PASS  cargo fmt -- --check
PASS  cargo test (5 passed, 0 failed)
PASS  cargo clippy --all-targets -- -D warnings
PASS  cargo build --release
PASS  Controlled loopback TCP transfer reports bytes or an honest Windows
      permission warning
PASS  Native UI Automation verified Download/Upload columns, N/A behavior,
      destination inspector, and encrypted-content boundary notice
PASS  Root executable matches the Cargo release artifact by SHA-256

The compiled root executable is excluded from Git while remaining in the project
folder for direct use. This release is not code-signed and has no custom icon.
