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
publication, and maps provider-neutral completion calls to an exact model client.
Keys are released one operation at a time by `ChannelKeyAuthority`; provider
credentials and selection remain behind `ModelProviderAuthority`. Neither secret
boundary is visible to the payload-blind dispatcher or orchestration core.

`ExactModelProviderRegistry` supplies an immutable exact-selector implementation
for already-constructed clients. Provider and channel output is validated against
the public wire bounds before it can turn into an authenticated-session framing
failure.

The production daemon still injects the unavailable service because its closed
configuration and pipeline-wiring APIs do not yet provision channel custody or
model providers. A later composition PR must add those explicit operator-owned
inputs rather than inventing secret storage or a default network endpoint here.

## Validation

```sh
cargo test -p chief-of-staff-host-data-plane -- --nocapture
cargo test -p chief-of-staff-host-control-protocol -- --nocapture
cargo clippy -p chief-of-staff-host-data-plane --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host-data-plane --no-deps
```
