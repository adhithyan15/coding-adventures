# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-18

### Added

- Initial storage-backed D18D tool audit store.
- JSON encoding/decoding for payload-free `ToolAuditRecord` rows.
- Query and inventory helpers over persisted audit records.
- Deterministic checkpoint pages for incremental audit replay.
- Timestamp and call-id scalar accessors for replay checkpoints.
- Durable named checkpoint state for supervisors that resume audit replay.
- Next-checkpoint timestamp and call-id scalar accessors for checkpoint pages.
- Timestamp and call-id scalar accessors for stored checkpoint state.
- Checkpointed replay helpers that deliver bounded pages into audit sinks and
  advance durable named checkpoints.
- Starting and next checkpoint scalar accessors for checkpoint replay summaries.
- Read-only supervisor checkpoint status inspection before draining.
- Starting and next checkpoint scalar accessors for supervisor checkpoint
  status.
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
- Typed status helpers on supervisor drain outcomes.
- Scheduler-action labels and intent helpers on supervisor drain outcomes.
- Outcome status helpers on supervisor drain reports and summaries.
- Flattened outcome status flags for supervisor drain run summaries.
- Flattened supervisor drain run summaries for host logs and scheduler loops.
- Typed classifier accessors for supervisor drain run summaries.
- Workload accessors for supervisor drain run reports.
- Workload accessors for supervisor drain run summaries.
- Last-checkpoint scalar accessors for supervisor drain run reports and
  summaries.
- Flattened last-checkpoint timestamp and call-id fields in supervisor drain
  run summaries.
- Flattened last-checkpoint presence flags in supervisor drain run summaries.
- Last-checkpoint consistency flags for presence, timestamp, call-id, and
  aggregate checkpoint fields in supervisor drain reports and summaries.
- Inventory count consistency flags for supervisor drain pages, plans, loops,
  reports, and summaries.
- Budget and run-status consistency flags for supervisor drain reports and
  summaries.
- Aggregate run-integrity drift flags and stable classifications for supervisor
  drain run reports and summaries.
- Starting and stored checkpoint boundary fields plus stored-checkpoint
  consistency flags for supervisor drain reports and summaries.
- Checkpoint-advance consistency flags that compare starting and last
  checkpoint movement in supervisor drain reports and summaries.
- Planned-vs-actual checkpoint alignment fields for supervisor drain preflight
  and result boundary checks.
- Stable checkpoint-drift classifications and investigation flags for
  supervisor drain run reports and summaries.
- Flattened planned and replayed follow-up row counts in supervisor drain run
  summaries.
- Follow-up pressure drift flags in supervisor drain run reports and summaries.
- Signed replayed-minus-planned row and follow-up pressure deltas in supervisor
  drain run summaries.
- Delta-direction helpers for supervisor drain run reports.
- Flattened row-count delta direction flags for supervisor drain run summaries.
- Flattened follow-up pressure delta direction flags for supervisor drain run
  summaries.
- Status aliases for supervisor drain run reports.
- Row-count match helpers for supervisor drain run reports and summaries.
- Flattened row-count and follow-up-pressure match alias flags for supervisor
  drain run summaries.
- Planned-count match and drift flag accessors for supervisor drain run
  summaries.
- Run-status accessors for supervisor drain run summaries.
- Remaining tick-budget helpers for supervisor drain plans, reports, and
  summaries.
- Per-page drain, continuation, and checkpoint-advance helpers for supervisor
  drain plans.
- Starting and next checkpoint scalar accessors for supervisor drain plan pages.
- Flattened idle, progress, and continuation flags in supervisor drain run
  summaries.
- Count-drift presence flags for supervisor drain run reports and summaries.
- Flattened aggregate count-drift flags for supervisor drain run summaries.
- Stable count-drift classifications for supervisor drain run reports and
  summaries.
- Flattened count-drift and host-investigation labels in supervisor drain run
  summaries.
- Count-drift investigation flags for supervisor drain run reports and
  summaries.
- No-count-drift helpers for supervisor drain run reports and summaries.
- Flattened no-count-drift flags for supervisor drain run summaries.
- Flattened plan-drift investigation flags for supervisor drain run summaries.
- Host-investigation flags for supervisor drain run reports and summaries.
- Stable host-investigation classifications for supervisor drain run reports
  and summaries.
- No-host-investigation helpers for supervisor drain run reports and summaries.
- Flattened no-host-investigation flags for supervisor drain run summaries.
- Host-investigation component helpers for supervisor drain run reports and
  summaries.
- Flattened host-investigation component flags for supervisor drain run
  summaries.
- Stable host-attention classifications for supervisor drain run reports and
  summaries.
- Flattened host-attention component flags for supervisor drain run summaries.
- Stable terminal-readiness classifications for supervisor drain run reports
  and summaries.
- Flattened terminal-readiness component flags for supervisor drain run
  summaries.
- Stable host-decision classifications for supervisor drain run reports and
  summaries.
- Flattened host-decision component flags for supervisor drain run summaries.
- Stable host-decision dashboard lanes and exact action counts for supervisor
  drain run reports and summaries.
- Stable host-decision dashboard priorities and sortable ranks for supervisor
  drain run reports and summaries.
- Stable host-decision readiness classifications and flattened routing flags
  for supervisor drain run reports and summaries.
- Stable host-decision route targets for scheduler, follow-up, investigation,
  triage, and no-route supervisor drain run queues.
- Aggregate host-decision and host-routing classifier label-integrity flags for
  supervisor drain run reports and summaries.
- Stable batch-write, replay, and inventory health labels with write
  count-integrity helpers for host logs.
- Stable checkpoint replay, checkpoint status, and planned page health labels
  with follow-up and count-integrity helpers for host logs.
- Typed component helpers for inventory, write, replay, checkpoint replay, and
  checkpoint page health classifications.
- Stable recommended action labels for inventory, write, replay, checkpoint
  replay, and checkpoint page health classifications.
- Stable route labels for inventory, write, replay, checkpoint replay, and
  checkpoint page health classifications.
- Stable priority and readiness labels for inventory, write, replay, checkpoint
  replay, checkpoint status, and planned page health classifications.
- Sortable lower-level health priority ranks plus manual-review,
  investigation, and triage readiness helpers for host dashboards.
- Aggregate preflight-plan and drain-result health dashboard fields on
  supervisor drain run summaries.
- Health dashboard label-integrity flags for aggregate supervisor plan/drain
  host queues.
- Combined supervisor plan/drain health-dashboard surfaces, route labels,
  priority ranks, readiness labels, and triage flags for one-key host queues.
- Flattened combined health-dashboard route, priority, and readiness booleans
  for supervisor drain run summary host queue filters.
- Aggregate health-label integrity helpers for inventory, write, replay,
  checkpoint replay, and checkpoint page summaries.
- Stable, parseable supervisor drain scheduler action recommendations for host
  loops.
- Report-level scheduler action labels for supervisor drain runs.
- Report-level scheduler action intent helpers for supervisor drain runs.
- Typed scheduler action intent helpers for continuation, follow-up routing, and
  plan-drift investigation branches.
- Flattened scheduler-action intent flags in supervisor drain run summaries.
- Flattened scheduler-action labels in supervisor drain run summaries.
- Flattened outcome labels in supervisor drain run summaries.
- Flattened idle outcome flags in supervisor drain run summaries.
- Per-classifier and aggregate label-to-classifier match flags for supervisor
  drain outcome, scheduler action, count-drift, and host-investigation
  summaries.
- Flattened no-action scheduler flags in supervisor drain run summaries.
- Report-level no-action scheduler helpers for supervisor drain runs.
- Batch audit write summaries for host flush loops.
- Replay helpers for loading queried audit rows into any `ToolAuditSink`.
- `StorageToolAuditSink` for runtime audit emission through D18A storage with
  payload-free failure summaries and batch flush coverage.
- In-memory and local-folder persistence coverage.
