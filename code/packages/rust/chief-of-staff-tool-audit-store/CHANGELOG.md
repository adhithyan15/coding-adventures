# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-18

### Added

- Initial storage-backed D18D tool audit store.
- JSON encoding/decoding for payload-free `ToolAuditRecord` rows.
- Query and inventory helpers over persisted audit records.
- Deterministic checkpoint pages for incremental audit replay.
- Durable named checkpoint state for supervisors that resume audit replay.
- Checkpointed replay helpers that deliver bounded pages into audit sinks and
  advance durable named checkpoints.
- Supervisor drain summaries that expose progress, continuation, and follow-up
  signals for bounded replay ticks.
- Bounded supervisor drain loops that compose replay ticks until end-of-log or
  tick-budget exhaustion.
- Batch audit write summaries for host flush loops.
- Replay helpers for loading queried audit rows into any `ToolAuditSink`.
- `StorageToolAuditSink` for runtime audit emission through D18A storage with
  payload-free failure summaries and batch flush coverage.
- In-memory and local-folder persistence coverage.
