# chief-of-staff-process-supervisor

`chief-of-staff-process-supervisor` is the concrete OS-process authority for
D18 Chief hosts. It re-verifies a registered signed package immediately before
each spawn, owns and reaps the child, bootstraps a fresh UUID-v7 secure channel,
delivers the exact relevant public package trust over that authenticated channel,
obtains launch bindings from an injected manifest-blind pipeline authority, and
reports readiness and heartbeat only after the child receives both inputs and
independently verifies the package with that trust.
The single configured host executable receives a final reserved
`--package-runtime deno|skill` pair derived from that verified package snapshot,
giving the production host a fail-closed runtime-dispatch seam without ambient
environment or registry input.

The same authenticated session now carries the host data plane. Child-side
helpers serialize bounded channel receive/publish/acknowledge, provider-neutral
text completion, installed model-tool catalog discovery, tool-aware completion,
and separate model-tool execution exchanges. When a dispatcher is injected, the supervisor automatically
reauthorizes and answers each request before processing the next record. A manual
composition may instead retain one authenticated request per host until its
service adapter answers through `respond_data_plane`. Both paths preserve exact
correlation across real cross-platform process pipes without adding an
unauthenticated side channel or exposing payloads to the orchestration core.
If graceful termination arrives while the child is blocked on an exchange, the
child helper returns a distinct termination condition so a concrete host can exit
successfully instead of misclassifying shutdown as a protocol failure.

Every host has its own request budget (D18S S-K5): a token bucket, by
default a burst of 128 requests refilled at 64 a second on the injected
monotonic clock. A request over budget is answered at once with
`Failed { Unavailable }`, which hosts already treat as "idle, retry later".
It is never queued or dispatched. `with_request_budget` sets the budget, and
`rate_limited_requests` reports how many of a host's requests were refused.

When a host is ended for cause, the call that noticed it returns that error
once: `inspect`, `start` or the data-plane calls. The causes are a framing
or control error, a full write queue, or a host still `Starting` past the
bootstrap timeout. The host is already `Exited` by then. The next `start`
spawns a fresh instance. Ending a host also kills its process group, which
it leads. The kill always runs before the host is reaped.

Its keyring and X3DH identity are shared through owned `Arc` handles, and its
session source is `Send`, so the complete supervisor can move with the daemon's
threaded control plane without copying secret key material.

The crate implements the dependency-light service reconciler's
`HostSupervisor` interface. It deliberately leaves durable restart policy,
scheduling, backoff, and registry updates to the reconciler and runnable
orchestrator. The production storage-backed provider revalidates exact host
registration, immutable pipeline channel claims, active topology, and directional
membership before every spawn. A fail-closed provider remains available to
compositions that intentionally have no durable pipeline wiring.
Channel endpoint and LLM service implementations remain injected behind
`chief-of-staff-host-data-plane` rather than entering this process-authority crate.

### Per-agent channel brokers (D18S P2.6d-2b)

`with_channel_brokers(ChannelBrokers)` gives every agent with bound channels
its own broker process. It is opt-in; without it, channel requests go to the
dispatcher as before.

```text
start(host)
  ├─ BrokerBusy?      an old relay for this agent has not finished: retry later
  ├─ abandon_pending  the agent's pending reservations (no old broker can commit now)
  ├─ launch broker    verified binary, the agent's keys on 3..3+n, Ready checked
  └─ spawn host       its Receive/Publish/Acknowledge go to the broker's relay

host ends   ─▶ broker killed, reaped, relay stopped
broker ends ─▶ host ended (inspect returns ProcessSupervisorError::Broker once)
```

Non-channel requests (completions, tool calls) go to the host's own
dispatch worker in the same way (P2.6d-4), so one slow request never holds
the supervisor's thread, which every host shares.

The broker's relay thread answers the host itself, through the shared
`HostLink`. That way a channel operation does not wait for the next refresh.
Encrypting and queueing a response happen under one lock, so frames reach
the host in the order the secure channel numbered them.

## Validation

```sh
cargo test -p chief-of-staff-process-supervisor -- --nocapture
cargo clippy -p chief-of-staff-process-supervisor --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-process-supervisor --no-deps
```
