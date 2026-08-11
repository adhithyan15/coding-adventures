# Changelog

## Unreleased

- Extended the real-process PTY audit ceremony through an invalid item-create
  prompt and exact trace selection of its durable failed event.
- Exposed authenticated `audit list` and `audit show TRACE`, and extended the
  real-process PTY suite through trace selection plus later verification that
  both audit-history accesses became durable.
- Exposed `audit enable` and extended the real-process PTY suite through audit
  activation, an invalid edit prompt, and later verification of its event.
- Exposed reversible item delete/restore and extended the real-process PTY
  suite through tombstone observation and exact historical restoration.
- Exposed redacted revision history listing and extended the PTY suite through
  canonical newest-first history after a durable edit.
- Exposed revision-safe login edit and extended the PTY restart suite through
  replacement plus a later redacted show.
- Exposed login add and redacted list/show through the thin executable.
- Extended the PTY suite through encrypted item persistence across processes.
- Exposed authenticated `audit verify` and `doctor --unlock` through the thin
  executable.
- Extended the real-process PTY suite across restart and redirected-stdin
  injection for both authenticated commands.

## 0.1.0

- Added the `vault-pm` executable composition root.
- Added real-process pseudo-terminal initialization and restart coverage.
