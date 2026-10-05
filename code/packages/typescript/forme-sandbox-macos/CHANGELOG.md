# Changelog

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

## 0.1.0 — 2026-10-01

- Added the `forme-macos-v1` exact-snapshot native launcher.
- Added deny-default Seatbelt enforcement, rlimits, trusted RSS monitoring,
  single-process policy, and private readiness attestation.
- Added passing platform probes for filesystem, network, process, memory, and fd denial.
- Added post-Seatbelt digest verification, explicit snapshot write denial,
  trusted runtime dependency roots, RSS/wall-clock watchdogs, and a stable
  `kqueue` parent-death watcher.
- Added deterministic Homebrew runtime-root derivation coverage.
- Limited loaded-image discovery to macOS so cross-platform policy validation
  does not collect an unused, slow host diagnostic report.
