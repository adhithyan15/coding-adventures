# chief-of-staff-host-data-plane

`chief-of-staff-host-data-plane` is the manifest-blind authorization boundary
between an authenticated child request and injected channel/model services.
Every request reloads the exact durable pipeline binding, registration, immutable
channel claims, active topology, and directional membership. Receive and
acknowledge require a read binding, publish requires a write binding, and model
calls must exactly match the launch-time selector, temperature, and token cap.

The dispatcher redacts all failures into the stable host-control taxonomy and
validates service responses before they re-enter the authenticated session. The
production daemon currently injects an unavailable service after authorization;
concrete channel-key custody and model-provider composition can replace that
service without weakening this boundary or changing process supervision.

## Validation

```sh
cargo test -p chief-of-staff-host-data-plane -- --nocapture
cargo clippy -p chief-of-staff-host-data-plane --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host-data-plane --no-deps
```
