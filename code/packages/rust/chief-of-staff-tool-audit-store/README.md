# chief-of-staff-tool-audit-store

`chief-of-staff-tool-audit-store` persists D18D payload-free
`ToolAuditRecord` rows through the D18A `StorageBackend` abstraction.

The crate keeps the boundary narrow:

- `chief-of-staff-tool-api` owns the audit record vocabulary
- this crate serializes those records into storage records
- callers can use in-memory, local-folder, or future storage backends without
  changing audit code
- runtimes can use `StorageToolAuditSink` to persist through the existing
  `ToolAuditSink` boundary and inspect storage failures after a call or batch
- hosts can replay queried audit rows into any existing audit sink for
  payload-free read-model rebuilds
- hosts can flush batches of audit rows and get a payload-free write summary
  with call-id-level storage failures
- replay checkpoints expose timestamp and call-id scalar accessors for host
  checkpoint logs
- supervisors can read deterministic checkpoint pages to resume audit replay
  after restarts without reprocessing the whole store
- checkpoint pages expose next-checkpoint timestamp and call-id scalar
  accessors for host replay logs
- supervisors can persist named replay checkpoints through the same D18A
  storage backend and advance them without regressing reader state
- stored checkpoints expose timestamp and call-id scalar accessors for host
  state logs
- supervisors can replay bounded pages from named checkpoints into audit sinks
  and advance the durable cursor after delivery
- checkpoint replay summaries expose starting and next checkpoint scalar
  accessors for host replay logs
- supervisors can inspect named checkpoint status before draining without
  advancing durable cursor state
- supervisor checkpoint status exposes starting and next checkpoint scalar
  accessors for host status logs
- supervisors can plan bounded drain pages without emitting rows, so schedulers
  can preview workload and follow-up pressure before committing a tick
- supervisors can drain one checkpointed page per tick and inspect progress,
  continuation, and follow-up signals without loading payloads
- supervisor inventory summaries include payload-free follow-up row counts so
  hosts can size follow-up pressure without inspecting audit payloads
- supervisors can run bounded drain loops that stop at end-of-log or report
  tick-budget exhaustion for the next scheduler pass
- supervisors can capture a preflight drain plan beside the actual bounded
  drain result, letting schedulers compare expected and delivered audit work
- supervisor drain reports classify scheduler outcomes as idle, caught up,
  needing continuation, needing follow-up, or diverged from preflight
- supervisor drain outcomes expose stable, parseable snake_case labels and
  action flags for host logs and scheduling decisions
- supervisor drain outcomes expose typed status helpers for host branching
- supervisor drain outcomes expose scheduler-action labels and typed intent
  helpers for host scheduling decisions
- supervisor drain reports and summaries expose outcome status helpers for host
  branching
- supervisor drain reports can emit flattened payload-free run summaries for
  host logs, schedulers, and continuation decisions
- supervisor drain run summaries expose typed classifier accessors for host
  scheduler and investigation decisions
- supervisor drain reports and summaries expose workload accessors for checkpoint,
  budget, planned, replayed, and delta fields
- supervisor drain reports and summaries expose last-checkpoint scalar accessors
  for timestamp and call-id host logs
- supervisor drain run summaries flatten last-checkpoint timestamp and call-id
  fields for payload-free checkpoint logging
- supervisor drain run summaries flatten planned and replayed follow-up row
  counts for host routing decisions
- supervisor drain run summaries flag when planned and replayed follow-up
  pressure counts diverge
- supervisor drain run summaries expose signed replayed-minus-planned row and
  follow-up pressure deltas for host logs
- supervisor drain reports and summaries expose delta-direction helpers for
  extra or missed replayed work and follow-up pressure
- supervisor drain reports expose status aliases for idle runs and matched
  follow-up pressure
- supervisor drain reports and summaries expose row-count match helpers for
  host scheduler checks
- supervisor drain run summaries expose exact planned-count match and drift
  flag accessors for host branching
- supervisor drain run summaries expose typed run-status accessors for
  continuation, follow-up, checkpoint, and budget decisions
- supervisor drain plans, reports, and summaries expose remaining tick-budget
  helpers for host scheduler budget logs
- supervisor drain plan pages expose per-page drain, continuation, and
  checkpoint-advance helpers for host preflight scheduling
- supervisor drain plan pages expose starting and next checkpoint scalar
  accessors for host preflight logs
- supervisor drain run summaries flatten idle, progress, and continuation flags
  for host log entries
- supervisor drain run summaries expose count-drift presence flags beside
  signed deltas for host logs
- supervisor drain run summaries expose stable count-drift classifications so
  hosts can distinguish row, follow-up pressure, and combined drift
- supervisor drain run summaries flatten stable count-drift and
  host-investigation labels beside their typed classifications
- supervisor drain reports and summaries expose explicit count-drift
  investigation flags for host schedulers
- supervisor drain reports and summaries expose no-count-drift helpers for
  aligned planned-vs-replayed counts
- supervisor drain summaries flatten plan-drift investigation flags beside
  count- and host-investigation flags
- supervisor drain reports and summaries expose host-investigation flags that
  combine plan-drift and count-drift signals
- supervisor drain reports and summaries expose stable host-investigation
  classifications for plan drift, count drift, or combined drift
- supervisor drain reports and summaries expose no-host-investigation helpers
  for terminal log branches
- supervisor drain run summaries flatten no-host-investigation flags for
  payload-free terminal host branches
- supervisor drain reports and summaries expose host-investigation component
  helpers for plan-drift and count-drift branches
- supervisor drain run summaries flatten host-investigation component flags for
  payload-free host logs
- supervisor drain reports and summaries expose stable, parseable scheduler action
  recommendations for continuation, follow-up routing, and plan-drift
  investigation
- scheduler action recommendations expose typed intent helpers on reports and
  summaries so hosts can
  branch on continuation, follow-up routing, or plan-drift investigation
  without parsing labels
- supervisor drain run summaries flatten scheduler-action intent flags for
  host logs and scheduling dashboards
- supervisor drain run summaries flatten stable scheduler-action labels beside
  the typed recommendation
- supervisor drain run summaries flatten stable outcome labels beside the typed
  run outcome
- supervisor drain reports and summaries expose no-action scheduler helpers for
  terminal host log entries

## Development

```bash
bash BUILD
```
