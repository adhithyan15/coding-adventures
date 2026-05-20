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
- Read-only supervisor checkpoint status inspection before draining.
- Read-only bounded supervisor drain plans that preview pages and follow-up
  pressure without advancing durable checkpoints.
- Payload-free follow-up row counts in audit inventory summaries for host
  schedulers.
- Supervisor drain summaries that expose progress, continuation, and follow-up
  signals for bounded replay ticks.
- Bounded supervisor drain loops that compose replay ticks until end-of-log or
  tick-budget exhaustion.
- Supervisor drain run reports that keep read-only preflight plans beside the
  actual bounded drain result.
- Scheduler-facing supervisor drain outcomes for idle, caught-up,
  continuation, follow-up, and plan-divergence states.
- Stable, parseable supervisor drain outcome labels and action flags for host
  schedulers.
- Flattened supervisor drain run summaries for host logs and scheduler loops.
- Typed classifier accessors for supervisor drain run summaries.
- Workload accessors for supervisor drain run reports.
- Workload accessors for supervisor drain run summaries.
- Last-checkpoint scalar accessors for supervisor drain run reports and
  summaries.
- Flattened last-checkpoint timestamp and call-id fields in supervisor drain
  run summaries.
- Flattened planned and replayed follow-up row counts in supervisor drain run
  summaries.
- Follow-up pressure drift flags in supervisor drain run reports and summaries.
- Signed replayed-minus-planned row and follow-up pressure deltas in supervisor
  drain run summaries.
- Delta-direction helpers for supervisor drain run reports.
- Status aliases for supervisor drain run reports.
- Row-count match helpers for supervisor drain run reports and summaries.
- Planned-count match and drift flag accessors for supervisor drain run
  summaries.
- Run-status accessors for supervisor drain run summaries.
- Count-drift presence flags for supervisor drain run reports and summaries.
- Stable count-drift classifications for supervisor drain run reports and
  summaries.
- Flattened count-drift and host-investigation labels in supervisor drain run
  summaries.
- Count-drift investigation flags for supervisor drain run reports and
  summaries.
- Flattened plan-drift investigation flags for supervisor drain run summaries.
- Host-investigation flags for supervisor drain run reports and summaries.
- Stable host-investigation classifications for supervisor drain run reports
  and summaries.
- Stable, parseable supervisor drain scheduler action recommendations for host
  loops.
- Report-level scheduler action labels for supervisor drain runs.
- Report-level scheduler action intent helpers for supervisor drain runs.
- Typed scheduler action intent helpers for continuation, follow-up routing, and
  plan-drift investigation branches.
- Flattened scheduler-action intent flags in supervisor drain run summaries.
- Flattened scheduler-action labels in supervisor drain run summaries.
- Flattened outcome labels in supervisor drain run summaries.
- Flattened no-action scheduler flags in supervisor drain run summaries.
- Report-level no-action scheduler helpers for supervisor drain runs.
- Batch audit write summaries for host flush loops.
- Replay helpers for loading queried audit rows into any `ToolAuditSink`.
- `StorageToolAuditSink` for runtime audit emission through D18A storage with
  payload-free failure summaries and batch flush coverage.
- In-memory and local-folder persistence coverage.
