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
- command validation against entity capabilities and command modes
- grant-backed command authorization for Chief of Staff agents and sandboxed
  tools
- registry-backed authorization decision auditing for accepted and rejected
  authorized commands
- registry-backed tool authorization decisions for Chief of Staff tool calls
- D18D-facing read tool facade for listing bridges/devices, reading entity
  state, describing capabilities, and inspecting bridge health without invoking
  integrations
- D18D-facing command tool facade for authorized `smart_home.command` calls that
  validate tool grants, command grants, optimistic state, and audit decisions
- accepted command results that remain separate from confirmed device state
- optimistic command state with expiry into stale snapshots
- desired-state reconciliation that detects missing, stale, or drifted state
  and reissues corrective commands
- deterministic supervision ticks that run optimistic expiry, desired-state
  reconciliation, and worker restart checks together
- replay of device events into the registry-backed state cache
- bridge health reports that update health without removing identities
- supervised bridge-worker heartbeat tracking and restart signals
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
