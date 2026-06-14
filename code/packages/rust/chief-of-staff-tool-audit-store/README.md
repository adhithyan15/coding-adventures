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
- batch write, replay, and inventory summaries expose stable health and
  follow-up labels plus count-integrity helpers for host logs
- replay checkpoints expose timestamp and call-id scalar accessors for host
  checkpoint logs
- checkpoint replay, checkpoint status, and planned drain pages expose stable
  health labels plus follow-up and count-integrity helpers for host logs
- inventory, write, replay, checkpoint replay, and checkpoint page health
  classifications expose typed component helpers for payload-free host
  branching
- inventory, write, replay, checkpoint replay, and checkpoint page health
  classifications expose stable recommended action labels for host routing
- inventory, write, replay, checkpoint replay, and checkpoint page health
  classifications expose stable route labels for host health queues
- inventory, write, replay, checkpoint replay, and checkpoint page summaries
  expose aggregate health-label integrity helpers for host-log validation
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
- supervisor drain outcomes expose stable key, digest, action, and route
  rollups for host-log grouping and label-integrity checks
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
- supervisor drain run summaries flatten last-checkpoint presence flags beside
  timestamp and call-id checkpoint fields
- supervisor drain reports and summaries expose last-checkpoint consistency
  flags for presence, timestamp, call-id, and aggregate checkpoint fields
- supervisor drain pages, plans, loops, reports, and summaries expose
  inventory count consistency flags for host-log integrity checks
- supervisor drain reports and summaries expose budget and run-status
  consistency flags for flattened host-log fields
- supervisor drain reports and summaries expose aggregate run-integrity drift
  flags and stable classifications for host-log integrity checks
- supervisor drain reports and summaries expose starting and stored checkpoint
  boundary fields plus stored-checkpoint consistency flags for host logs
- supervisor drain reports and summaries expose checkpoint-advance consistency
  flags that compare starting and last checkpoint movement for host logs
- supervisor drain reports and summaries expose planned-vs-actual checkpoint
  alignment fields for preflight/drain boundary checks
- supervisor drain reports and summaries expose stable checkpoint-drift
  classifications and investigation flags for host logs
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
- supervisor drain run summaries flatten row-count and follow-up-pressure match
  alias flags for host scheduler checks
- supervisor drain run summaries expose exact planned-count match and drift
  flag accessors for host branching
- supervisor drain run summaries expose typed run-status accessors for
  continuation, follow-up, checkpoint, and budget decisions
- supervisor drain plans, reports, and summaries expose remaining tick-budget
  helpers for host scheduler budget logs
- supervisor drain plans, loops, reports, and summaries expose stable
  tick-budget usage classifications and labels for host scheduler logs
- supervisor drain plans, loops, reports, and summaries expose stable
  tick-budget pressure labels for slack, exact-boundary, and continuation
  scheduling decisions
- supervisor drain reports and summaries flatten tick-budget pressure route,
  priority, readiness, and queue-key labels for host scheduler queues
- supervisor drain reports and summaries flatten tick-budget pressure rollup
  keys and queue digests for compact host-log grouping and drift checks
- supervisor drain reports and summaries flatten tick-budget pressure route
  keys and route digests that bind route buckets back to queue digests for
  compact host-log routing groups
- supervisor drain reports and summaries flatten tick-budget pressure
  route-digest queue keys and queue digests for priority/readiness host-log
  grouping and drift checks
- supervisor drain reports and summaries expose route-digest queue key and
  digest outcome helpers for settled/log-only/action routing decisions
- supervisor drain plan pages expose per-page drain, continuation, and
  checkpoint-advance helpers for host preflight scheduling
- supervisor drain plan pages expose starting and next checkpoint scalar
  accessors for host preflight logs
- supervisor drain run summaries flatten idle, progress, and continuation flags
  for host log entries
- supervisor drain run summaries expose count-drift presence flags beside
  signed deltas for host logs
- supervisor drain run summaries flatten row-count delta direction flags beside
  signed deltas for host logs
- supervisor drain run summaries flatten follow-up pressure delta direction
  flags beside signed deltas for host logs
- supervisor drain run summaries flatten aggregate count-drift flags for
  payload-free host logs
- supervisor drain run summaries expose stable count-drift classifications so
  hosts can distinguish row, follow-up pressure, and combined drift
- supervisor drain run summaries flatten stable count-drift and
  host-investigation labels beside their typed classifications
- supervisor drain reports and summaries expose explicit count-drift
  investigation flags for host schedulers
- supervisor drain reports and summaries expose no-count-drift helpers for
  aligned planned-vs-replayed counts
- supervisor drain run summaries flatten no-count-drift flags beside stable
  count-drift classifications
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
- supervisor drain reports and summaries expose host-attention classifications
  that combine scheduler action, host investigation, and run-integrity signals
- supervisor drain run summaries flatten host-attention component flags for
  payload-free host dashboards
- supervisor drain reports and summaries expose terminal-readiness
  classifications that distinguish idle-ready, caught-up-ready, and pending
  host-attention states
- supervisor drain run summaries flatten terminal-readiness component flags for
  payload-free terminal dashboards
- supervisor drain reports and summaries expose host-decision classifications
  that pick the next scheduler, follow-up, drift, or integrity action
- supervisor drain run summaries flatten exact host-decision component flags for
  payload-free host action dashboards
- supervisor drain reports and summaries expose host-decision dashboard lanes
  and action counts for grouping terminal, scheduler, follow-up, investigation,
  and mixed-action work
- supervisor drain reports and summaries expose host-decision dashboard
  priorities and sortable ranks for queueing routine action, drift
  investigation, integrity investigation, and mixed-action work
- supervisor drain reports and summaries expose host-decision readiness
  classifications and flattened routing flags for settled, routine,
  investigation, integrity, manual-review, and triage queues
- supervisor drain reports and summaries expose host-decision route targets
  for scheduler, follow-up, drift-investigation, integrity-investigation,
  triage, and no-route queues
- supervisor drain reports and summaries expose stable host-decision queue keys
  for grouping decision kind, lane, route, priority, and readiness in one host
  action key
- supervisor drain run summaries flatten host-decision queue-key integrity flags
  so host logs can verify each key component against the typed decision
- supervisor drain reports and summaries expose aggregate host-decision and
  host-routing classifier label-integrity flags for payload-free dashboards
- inventory, write, replay, checkpoint replay, checkpoint status, and planned
  page health summaries expose stable priority and readiness labels for
  host-log dashboards
- lower-level audit health priority/readiness helpers provide sortable ranks,
  manual-review, investigation, and triage signals without parsing labels
- supervisor drain run summaries flatten aggregate preflight-plan and
  drain-result health dashboard labels, ranks, readiness, and label-integrity
  flags for host queues
- supervisor drain run summaries flatten preflight-plan and drain-result health
  action component flags and counts for per-side scheduler filters
- supervisor drain run summaries flatten preflight-plan and drain-result health
  route, priority, and readiness booleans for per-side queue filters
- supervisor drain run summaries expose combined plan/drain health-dashboard
  surfaces, routes, ranks, readiness, and triage flags for one-key host queues
- supervisor drain run summaries flatten combined health-dashboard action
  component flags and counts for one-key scheduler filters
- supervisor drain run summaries expose stable combined health-dashboard action
  lanes and labels for host dashboard grouping
- supervisor drain run summaries expose stable combined health-dashboard queue
  keys for grouping action lane, route, priority, and readiness in one host key
- supervisor drain run summaries flatten combined health-dashboard queue-key
  integrity flags so host logs can verify each key component against the digest
- supervisor drain run summaries flatten combined health-dashboard route,
  priority, and readiness booleans for host queue filters
- supervisor drain run summaries expose host-run queue rollups that combine
  health-dashboard and host-decision action surfaces, priority ranks,
  readiness flags, and queue-key integrity checks for host logs
- supervisor drain reports and summaries expose stable composite host-run queue
  keys that combine health-dashboard and host-decision keys for host logs
- supervisor drain reports and summaries expose aggregate host-run queue routes,
  readiness labels, and label-integrity flags for host log grouping
- supervisor drain reports and summaries expose aggregate host-run action lanes,
  component flags, counts, and label-integrity flags for host queue grouping
- supervisor drain reports and summaries expose aggregate host-run queue
  priorities and compact route/lane/priority/readiness grouping keys
- supervisor drain reports and summaries expose aggregate host-run queue digest
  label-integrity flags so host logs can verify queue keys, routes, lanes,
  priorities, readiness, and grouping labels together
- supervisor drain reports and summaries expose host-run attention component
  flags and counts for health-dashboard, scheduler, host-investigation, and
  run-integrity work
- supervisor drain reports and summaries expose stable aggregate host-run
  attention labels and label-integrity flags for grouping top-level host work
- supervisor drain reports and summaries expose compact host-run supervision
  keys that combine top-level attention and queue grouping for dashboard rows
- supervisor drain run summaries flatten host-run supervision routing,
  priority, review, investigation, triage, and label-integrity flags
- supervisor drain reports and summaries expose compact host-run escalation
  labels, ranks, routing flags, and label-integrity checks for supervision
  queues
- supervisor drain reports and summaries expose compact host-run escalation
  digest labels that bind escalation classifications back to supervision keys
  for host-log integrity checks
- supervisor drain reports and summaries expose host-run escalation route
  labels and digest-match flags for queue routing and host-log integrity checks
- supervisor drain reports and summaries expose compact host-run escalation
  route-key labels that bind routes back to escalation digests for queue
  grouping and host-log integrity checks
- supervisor drain reports and summaries expose compact host-run escalation
  queue keys that add priority and readiness labels to route keys for stable
  queue grouping and integrity checks
- supervisor drain reports and summaries flatten host-run escalation queue
  settled, action, rank, routing, review, investigation, triage, and route
  presence flags for host dashboards
- supervisor drain reports and summaries expose compact host-run escalation
  rollup keys that group escalation kind, route, priority, and readiness for
  concise queue dashboards
- supervisor drain reports and summaries flatten host-run escalation rollup
  integrity and routing flags beside the expanded escalation queue key
- supervisor drain reports and summaries expose host-run escalation dashboard
  keys that bind compact rollups back to source supervision keys for
  dashboard integrity checks
- supervisor drain reports and summaries expose stable host-run escalation
  dashboard lanes and lane-integrity flags for compact queue filtering
- supervisor drain reports and summaries expose compact host-run escalation
  dashboard lane-rollup keys that group lane, route, priority, and readiness
  for dashboard rows and integrity checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane digests that bind lane classifiers back to their compact rollup keys for
  host-log grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue keys that bind lane digests to route, priority, and readiness
  queues for host-log grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue digests that bind queue keys back to expanded dashboard keys for
  host-log grouping and drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue route flags for routing lane queues and their digests to routine
  action, manual review, investigation, integrity investigation, or triage
  queues
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue classifier labels for route, priority, and readiness on queue keys
  and queue digests
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-lane labels that collapse queue keys and digests into
  settled, auto-route, review, investigation, integrity investigation, or
  triage queues
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-lane status flags and priority ranks for queue keys and
  queue digests
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action keys and action digests that bind action lanes back to
  queue keys and queue digests for host-log grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action rollup keys and digests that group action lane, route,
  priority, and readiness for compact host-log buckets and drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup digest classifier labels for host-log filters and
  label drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup digest status, priority, and route flags for compact
  host-log queues
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route keys that bind compact route buckets back to
  action-rollup digests for host-log grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route digests that bind route keys back to source
  action-rollup digests for stable host-log route grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue keys that bind route digests to
  queue priority and readiness for stable host-log queue routing and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue digests that bind queue keys back
  to source route digests for stable host-log queue grouping and drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action-lane route, priority,
  readiness, and classifier label-integrity flags across queue-key and digest
  dashboards
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue-digest action lanes, labels,
  status flags, and drift checks for compact host-log routing filters
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action keys and action digests
  that bind queue action lanes back to queue keys and queue digests for
  host-log grouping and drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action digest classifier labels,
  status flags, and route targets for compact host-log routing filters
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route keys and digests
  that bind action digests back to route buckets for compact host-log routing
  and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue keys and
  digests that bind action-route digests to priority/readiness queues for
  compact host-log routing and drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue
  action-lane labels, status flags, and route targets for compact host-log
  routing filters
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue
  action-lane readiness gates and priority ranks for queue-key and digest
  dashboards
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue
  action-lane route labels and route-target flags for queue-key and digest
  dashboards
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue
  action-lane priority/readiness classifiers and labels for queue-key and
  digest dashboards
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue
  action-lane classifier label-integrity flags across queue-key and digest
  dashboards
- supervisor drain reports and summaries bind host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue digest
  action-lane labels back to their queue keys and source digests for host-log
  drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  keys and action digests that bind queue action lanes back to queue keys and
  queue digests with label-integrity fields for host-log grouping and drift
  checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  digest classifier labels, status flags, and route targets for compact
  host-log routing filters
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route keys and route digests that bind action digests back to route buckets
  for compact host-log routing and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route-digest queue keys and queue digests for priority/readiness grouping and
  drift checks
- supervisor drain reports and summaries flatten host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route-digest queue action digest classifier labels, status flags, and route
  targets for compact host-log routing filters
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route-digest queue action route keys and route digests that bind deep action
  digests back to route buckets for compact host-log routing and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route-digest queue action route-digest queue keys and queue digests for deep
  route priority/readiness grouping and drift checks
- supervisor drain reports and summaries expose host-run escalation dashboard
  lane queue action-rollup route-digest queue action route-digest queue action
  route-digest queue action route-digest queue action keys and action digests
  that bind deep route queue lanes back to queue keys and queue digests for
  host-log grouping and drift checks
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
- supervisor drain run summaries flatten outcome key, digest, action, and route
  rollups beside route labels and drift flags
- supervisor drain run summaries flatten idle outcome flags beside drain-idle
  status for host logs
- supervisor drain reports and summaries expose per-classifier and aggregate
  label-to-classifier match flags for outcome, scheduler action, count drift,
  checkpoint drift, and host-investigation logs
- supervisor drain run-integrity classifications expose their own
  label-to-classifier match flags for host-log integrity dashboards
- host-attention classifications expose their own label-to-classifier match
  flags for host-log routing dashboards
- terminal-readiness classifications expose their own label-to-classifier match
  flags for host-log terminal routing dashboards
- supervisor drain run summaries flatten outcome status flags for host logs and
  terminal scheduler dashboards
- supervisor drain reports and summaries expose no-action scheduler helpers for
  terminal host log entries

## Development

```bash
bash BUILD
```
