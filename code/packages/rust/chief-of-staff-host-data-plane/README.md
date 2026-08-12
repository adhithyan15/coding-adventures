# chief-of-staff-host-data-plane

`chief-of-staff-host-data-plane` is the manifest-blind authorization boundary
between an authenticated child request and injected channel/model services.
Every request reloads the exact durable pipeline binding, registration, immutable
channel claims, active topology, and directional membership. Receive and
acknowledge require a read binding, publish requires a write binding, and model
calls must exactly match the launch-time selector, temperature, and token cap.

The dispatcher redacts all failures into the stable host-control taxonomy and
validates service responses before they re-enter the authenticated session.

`AuthorityBackedHostDataPlaneService` is the concrete execution layer. It opens
the real durable encrypted receiver/originator endpoints, retains a bounded
receive-to-ack delivery ledger, provisions sealed receiver grants before a
publication, and maps provider-neutral text and tool-aware completion calls to an
exact model client. Tool declarations and replayable prior results remain model
inputs here; this layer does not authorize or execute model-emitted D18D calls.
Keys are released one operation at a time by `ChannelKeyAuthority`; provider
credentials and selection remain behind `ModelProviderAuthority`. Neither secret
boundary is visible to the payload-blind dispatcher or orchestration core.

`ExactModelProviderRegistry` supplies an immutable exact-selector implementation
for already-constructed clients. Provider and channel output is validated against
the public wire bounds before it can turn into an authenticated-session framing
failure.

`ExactChannelKeyAuthority` is the corresponding pre-composition key registry. It
consumes zeroizing secret owners, scopes every read or write key to one exact
pipeline, agent, and channel, rejects duplicate or cross-direction registrations,
and releases fresh short-lived crypto owners only when the current durable binding
matches all three identities. It deliberately does not read files or define a
vault format; those are replaceable provisioning adapters.

The production daemon injects this service only for a non-empty typed data-plane
configuration. It provisions explicit operator-owned channel-key files and exact
Ollama clients; absent or empty declarations retain the unavailable service. No
secret format or default network endpoint is invented here.

## Validation

```sh
cargo test -p chief-of-staff-host-data-plane -- --nocapture
cargo test -p chief-of-staff-host-control-protocol -- --nocapture
cargo clippy -p chief-of-staff-host-data-plane --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host-data-plane --no-deps
```
