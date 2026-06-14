# smart-home-runtime

Smart-home runtime coordinator for event routing, command validation, state cache, and supervision.

`smart-home-runtime` is the first layer above the normalized D23 model and the
in-memory registry. It is synchronous and deterministic so later actor systems,
transport workers, protocol bridges, and Chief of Staff tools can share the same
small set of runtime rules.

Included surfaces:

- in-process event bus with explicit subscriptions and filters
- replay checkpoints so subscribers can rebuild state from an earlier event-log
  position before receiving live updates
- read-side event-log and subscription backlog queries for dashboards, tools,
  and tests that need bounded runtime inspection
- compact event-log summaries for selected replay windows and event kinds
- typed command-result queries and summaries over checkpointed runtime event
  history
- bounded event-bus delivery peeking and draining for subscription polling
- compact delivery-batch summaries for subscription polling results without
  exposing event payloads
- explicit event-bus unsubscribe lifecycle that returns undelivered events and
  removes pending delivery state
- compact read-only snapshots for registry counts, event-bus backlog,
  supervisor pressure, pairing expiry, desired state, and stale cached state
- pending-work summaries derived from runtime read snapshots for D18D status
  tools
- event-bus backlog status helpers for distinguishing absent subscribers,
  caught-up streams, and backlogged streams
- event-bus aggregate pressure helpers for counting lagging subscriptions and
  maximum pending delivery depth
- event-bus lagging-subscription percentage helpers for read-side pressure
  thresholds
- event-bus pressure status helpers for classifying caught-up, partial, and
  fully backlogged subscriber fan-out
- subscription backlog status helpers for identifying lagging event-stream
  subscribers without draining their delivery queues
- subscription inventory summaries for read-side event-stream filter coverage
  and queue pressure checks
- runtime-owned discovery catalog recording that reconciles normalized
  discovery results into unpaired bridge candidates
- discovery worker-run ingest that records preferred batch results, reconciles
  accepted candidates into the registry, and returns inserted/replaced/ignored
  catalog counts
- runtime-owned discovery worker schedules with source/interface scope, due-run
  plans, run-status tracking, retry/backoff policy, and cadence advancement
  after scheduled ingest
- executable mDNS scan-plan projection from due discovery schedules into
  per-interface IPv4/IPv6 scan requests without mutating runtime state
- supervised mDNS discovery runs that mark due workers started, execute scan
  plans through an injectable runner, adapt reports into worker runs, and
  record scheduler/catalog outcomes
- read-side scheduled discovery worker snapshots for supervision observations,
  including due status, last run status, record/failure counts, catalog changes,
  retry policy, current retry delay, and consecutive failure pressure
- D18D-facing discover tool facade for authorized discovery reads, freshness
  filters, summaries, and bridge-candidate output
- D18D-facing read tool facade entries for listing scheduled discovery workers
  and reading compact discovery record/signal summaries without invoking scans
- read-side discovery pairing plans that combine recorded discovery signals with
  first-party integration pairing semantics before any pairing session starts
- composed event-bus health summaries for replay history, stream coverage, and
  current queue pressure checks
- command validation against entity capabilities and command modes
- grant-backed command authorization for Chief of Staff agents and sandboxed
  tools
- registry-backed authorization decision auditing for accepted and rejected
  authorized commands
- registry-backed tool authorization decisions for Chief of Staff tool calls
- read-side authorization-decision queries and summaries for Chief audit tools
- read-side capability-grant queries and summaries for Chief grant governance
  tools
- read-side room summaries and topology coverage derived from registry-owned
  devices, entities, cached states, and scenes
- D18D-facing read tool facade for listing bridges/devices/scenes, describing
  scenes, reading room topology summaries, reading aggregate topology coverage,
  reading entity state, describing capabilities, inspecting bridge health,
  listing event subscriptions, inspecting event-log entries, listing and
  summarizing command results, reading
  authorization decisions and summaries, reading capability grants and
  summaries, reading compact runtime snapshots, listing desired-state targets,
  listing pairing sessions, listing supervised bridge workers, reading worker
  heartbeat schedules, listing scheduled discovery workers, reading discovery
  summaries, reading discovery pairing plans, previewing supervision plans, and
  observing supervision status without invoking integrations
- D18D-facing subscribe tool facade that authorizes event-stream access and
  registers filtered replay subscriptions with checkpoints
- D18D-facing poll and unsubscribe tool facades that authorize event-stream
  reads, support bounded peek/drain delivery batches, and return undelivered
  events when subscriptions are retired
- D18D-facing pair-bridge facade with short-lived pairing sessions that complete
  only to Vault references and non-secret audit metadata, never raw credentials
- D18D-facing complete-pairing facade that authorizes VaultRef completion
  through the same pairing capability before mutating runtime session state
- D18D-facing report-event facade that authorizes adapter-observed device
  events and bridge-health reports before mutating registry state
- D18D-facing desired-state mutation facade that authorizes target set/clear
  operations before mutating runtime-owned reconciliation intent
- read-side queries for pairing sessions and desired-state supervision targets
- pairing-session inventory summaries for bridge-pairing status, expiry, and
  VaultRef completion counts
- D18D-facing command tool facade for authorized `smart_home.command` calls that
  validate tool grants, command grants, optimistic state, and audit decisions
- D18D-facing supervision tool facade for authorized desired-state
  reconciliation and full supervision ticks through runtime-owned mutation
  paths
- accepted command results that remain separate from confirmed device state
- optimistic command state with expiry into stale snapshots
- desired-state reconciliation that detects missing, stale, or drifted state
  and reissues corrective commands
- deterministic supervision ticks that run optimistic expiry, desired-state
  reconciliation, and worker restart checks together
- compact supervision tick summaries for read-side tools that need to compare
  planned work with actual pairing expiry, state expiry, reconciliation, and
  restart actions
- non-mutating supervision plans that preview due refreshes, desired-state
  drift, pairing expiry, bridge-worker restarts, and scheduled discovery worker
  runs before a tick performs any writes
- compact supervision plan summaries for read-side due-work counts without
  walking every planned target
- read-only supervision observations that combine due action counts with worker
  heartbeat schedules for Chief of Staff status tools
- replay of device events into the registry-backed state cache
- bridge health reports that update health without removing identities
- supervised bridge-worker heartbeat tracking and restart signals
- read-side worker queries by bridge, integration, status, restart count, and
  heartbeat deadline
- deterministic worker heartbeat deadline schedules for supervisor wakeups
- worker restart reconciliation that marks registered bridges degraded with
  health events
- deterministic worker restart plans that can be inspected before an actor,
  sandbox, or process runner performs the restart

## Dependencies

- smart-home-core
- smart-home-discovery
- smart-home-integration-catalog
- smart-home-registry

## Development

```bash
# Run tests
bash BUILD
```
