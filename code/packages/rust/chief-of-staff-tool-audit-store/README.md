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
- supervisors can read deterministic checkpoint pages to resume audit replay
  after restarts without reprocessing the whole store
- supervisors can persist named replay checkpoints through the same D18A
  storage backend and advance them without regressing reader state
- supervisors can replay bounded pages from named checkpoints into audit sinks
  and advance the durable cursor after delivery
- supervisors can inspect named checkpoint status before draining without
  advancing durable cursor state
- supervisors can plan bounded drain pages without emitting rows, so schedulers
  can preview workload and follow-up pressure before committing a tick
- supervisors can drain one checkpointed page per tick and inspect progress,
  continuation, and follow-up signals without loading payloads
- supervisors can run bounded drain loops that stop at end-of-log or report
  tick-budget exhaustion for the next scheduler pass
- supervisors can capture a preflight drain plan beside the actual bounded
  drain result, letting schedulers compare expected and delivered audit work
- supervisor drain reports classify scheduler outcomes as idle, caught up,
  needing continuation, needing follow-up, or diverged from preflight
- supervisor drain outcomes expose stable, parseable snake_case labels and
  action flags for host logs and scheduling decisions
- supervisor drain reports can emit flattened payload-free run summaries for
  host logs, schedulers, and continuation decisions
- supervisor drain summaries expose stable, parseable scheduler action
  recommendations for continuation, follow-up routing, and plan-drift
  investigation
- scheduler action recommendations expose typed intent helpers so hosts can
  branch on continuation, follow-up routing, or plan-drift investigation
  without parsing labels

## Development

```bash
bash BUILD
```
