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
- bounded event-bus delivery peeking and draining for subscription polling
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
- command validation against entity capabilities and command modes
- grant-backed command authorization for Chief of Staff agents and sandboxed
  tools
- registry-backed authorization decision auditing for accepted and rejected
  authorized commands
- registry-backed tool authorization decisions for Chief of Staff tool calls
- D18D-facing read tool facade for listing bridges/devices, reading entity
  state, describing capabilities, inspecting bridge health, and observing
  supervision status without invoking integrations
- D18D-facing subscribe tool facade that authorizes event-stream access and
  registers filtered replay subscriptions with checkpoints
- D18D-facing pair-bridge facade with short-lived pairing sessions that complete
  only to Vault references, never raw credentials
- read-side queries for pairing sessions and desired-state supervision targets
- pairing-session inventory summaries for bridge-pairing status, expiry, and
  VaultRef completion counts
- D18D-facing command tool facade for authorized `smart_home.command` calls that
  validate tool grants, command grants, optimistic state, and audit decisions
- accepted command results that remain separate from confirmed device state
- optimistic command state with expiry into stale snapshots
- desired-state reconciliation that detects missing, stale, or drifted state
  and reissues corrective commands
- deterministic supervision ticks that run optimistic expiry, desired-state
  reconciliation, and worker restart checks together
- non-mutating supervision plans that preview due refreshes, desired-state
  drift, pairing expiry, and worker restarts before a tick performs any writes
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
- smart-home-registry

## Development

```bash
# Run tests
bash BUILD
```
