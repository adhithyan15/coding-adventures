# chief-of-staff-host-control-protocol

`chief-of-staff-host-control-protocol` defines the minimum authenticated
lifecycle conversation between the D18 orchestrator and one spawned host. A
child independently verifies its signed package, sends `Ready(package_hash)`,
then sends heartbeats. The orchestrator alone sends `Terminate`.

The wrapper runs over `chief-of-staff-secure-host-channel`, enforces role and
lifecycle ordering, and fails closed after malformed, unauthenticated,
wrong-direction, replayed, or package-mismatched peer input. Heartbeats carry no
child-selected timestamp: the orchestrator attaches its trusted monotonic receive
time after authentication.

The crate owns no clock, file descriptor, process, stream, filesystem, or network
capability. A concrete process supervisor supplies complete encrypted frames and
receipt times.

## Validation

```sh
cargo test -p chief-of-staff-host-control-protocol -- --nocapture
cargo clippy -p chief-of-staff-host-control-protocol --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host-control-protocol --no-deps
```
