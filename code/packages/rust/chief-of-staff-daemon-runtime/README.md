# chief-of-staff-daemon-runtime

`chief-of-staff-daemon-runtime` turns the authenticated D18 Chief WebSocket API
into a continuously converging daemon. It runs one mandatory reconciliation
tick before serving, schedules bounded ticks through the API's serialized
control plane, and stops the listener if convergence fails.

The package intentionally does not parse configuration, load identities or
trusted keys, choose listener addresses, install OS services, or own secrets.
Those remain outer composition adapters.

## Validation

```sh
cargo test -p chief-of-staff-daemon-runtime -- --nocapture
cargo clippy -p chief-of-staff-daemon-runtime --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-runtime --no-deps
```
