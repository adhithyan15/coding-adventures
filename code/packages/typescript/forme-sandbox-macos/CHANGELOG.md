# Changelog

## 0.1.0 — 2026-10-01

- Added the `forme-macos-v1` exact-snapshot native launcher.
- Added deny-default Seatbelt enforcement, rlimits, trusted RSS monitoring,
  single-process policy, and private readiness attestation.
- Added passing platform probes for filesystem, network, process, memory, and fd denial.
- Added post-Seatbelt digest verification, explicit snapshot write denial,
  trusted runtime dependency roots, RSS/wall-clock watchdogs, and a stable
  `kqueue` parent-death watcher.
- Added deterministic Homebrew runtime-root derivation coverage.
