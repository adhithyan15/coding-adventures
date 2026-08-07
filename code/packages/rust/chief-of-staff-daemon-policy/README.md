# chief-of-staff-daemon-policy

`chief-of-staff-daemon-policy` supplies the first production policy adapters for
the D18 Chief daemon. A 256-bit bearer credential authenticates a loopback client
with constant-time comparison and zeroized storage. Successful authentication
creates an opaque connection-local operator session; all current lifecycle API
operations require that session.

Channel topology changes remain denied by default. The adapter deliberately does
not turn a local bearer credential into privilege approval: a later Trust Checker
must authorize the exact immutable topology mutation.

The package generates credential material but performs no filesystem, terminal,
environment, or network access. Outer composition owns protected persistence and
delivery to the CLI.

## Validation

```sh
cargo test -p chief-of-staff-daemon-policy -- --nocapture
cargo clippy -p chief-of-staff-daemon-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-daemon-policy --no-deps
```
