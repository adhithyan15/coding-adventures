# chief-of-staff-broker-launcher

The supervisor's half of an agent broker's life (D18S S-K1, S-K7, step 6
P2.6d-2a). It holds no channel key itself.

| Step | What it does |
|---|---|
| `BrokerKeyFiles::slots_for(binding)` | Maps the binding's channels, in order, to the key files the broker gets on descriptors 3..3+n. A bound channel without its keys is refused before anything is spawned. |
| `abandon_pending_on_write_channels` | Gives back any reservation a previous broker left on the agent's write channels. Call it only once that broker is killed and reaped, and its relay joined. |
| `launch(program, binding, keys, ..)` | Opens the key files (owner-only, never read here), execs the *verified descriptor* of the broker binary holding them at 3..3+n, sends Bootstrap, and checks `Ready`. Any failure kills and reaps the broker. |
| `check_ready` | `Ready`'s public keys must be exactly the slots' public halves, in order, and each must be the key the channel definition names for this agent. |
| `start_relay` | One thread per broker. It relays each channel request, answers each callback with the daemon's `CallbackServer`, and delivers the response through a `ResponseSink`. |

## The relay's rules

- **The deadline counts only the broker's time.** Time spent serving its
  callbacks, which is the daemon's storage work, is not charged to it.
- **The binding is pinned.** Callbacks re-resolve the binding every time,
  and `PinnedBindingResolver` accepts the result only if it names the same
  pipeline and agent the broker was launched for. A rewired host's old
  broker cannot act as the new agent.
- **The relay never blocks on the broker.** Writes go through a writer
  thread it only queues to. A broker that stops reading cannot hold the
  relay; it stops answering, and the deadline ends it.
- **Anything wrong ends the broker:** a violating callback, a missed
  deadline, a response that does not answer its request, a broken or
  out-of-order frame, an exit, or a host that is gone.

## Platforms

Verified launch needs `execveat`, so it is Linux only. Elsewhere
`VerifiedExecutable` is an empty type: no broker can be launched (S-P3).

## Tests

- `tests/units.rs`: slot order, missing, duplicate and cross-direction
  keys; abandon touches only write channels; the Ready check catches a
  wrong key, wrong order, a missing or extra key, an impostor, and a
  destroyed channel; the pinned resolver.
- `tests/relay.rs` plays the broker in process:
  - an honest round trip, repeated;
  - refused, each ending the relay: a violation, a missed deadline, a
    response for another request, a wrong operation, a broken frame, an
    unexpected Ready, an exit, a host that is gone, a non-channel request;
  - the deadline does not charge 400 ms of daemon time against a 300 ms
    deadline, but does charge the broker's own slowness;
  - at most one request queues behind the one in flight.
- The real broker binary, launched and relayed end to end, is tested in
  `chief-of-staff-agent-broker/tests/launched.rs`, where its binary is
  built.

```sh
cargo test -p chief-of-staff-broker-launcher
```
