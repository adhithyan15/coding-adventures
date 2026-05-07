# smart-home-runtime

Smart-home runtime coordinator for event routing, command validation, state cache, and supervision.

`smart-home-runtime` is the first layer above the normalized D23 model and the
in-memory registry. It is synchronous and deterministic so later actor systems,
transport workers, protocol bridges, and Chief of Staff tools can share the same
small set of runtime rules.

Included surfaces:

- in-process event bus with explicit subscriptions and filters
- command validation against entity capabilities and command modes
- accepted command results that remain separate from confirmed device state
- optimistic command state with expiry into stale snapshots
- desired-state reconciliation that detects missing, stale, or drifted state
  and reissues corrective commands
- deterministic supervision ticks that run optimistic expiry, desired-state
  reconciliation, and worker restart checks together
- replay of device events into the registry-backed state cache
- bridge health reports that update health without removing identities
- supervised bridge-worker heartbeat tracking and restart signals
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
