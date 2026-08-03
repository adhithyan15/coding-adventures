# chief-of-staff-service-registry

`chief-of-staff-service-registry` is the durable discovery cache for the D18
Chief orchestrator. Each record keeps the immutable signed-package identity and
restart intent needed after a crash together with the orchestrator's last
observation of one host process.

The registry deliberately is not a process supervisor and is not a second
channel database. The supervisor remains authoritative for live process state;
`chief-of-staff-channel-endpoints` remains authoritative for channel membership.
The cached process ID, lifecycle, heartbeat, and control-channel ID are evidence
that reconciliation must verify after restart.

Records use a bounded, versioned binary codec and live behind the repository's
`StorageBackend`. Registration uses atomic create-if-absent. Every update and
deregistration is revision-CAS guarded, preventing a stale reconciler from
overwriting or deleting a newer observation.

## Validation

```sh
cargo test -p chief-of-staff-service-registry -- --nocapture
cargo clippy -p chief-of-staff-service-registry --all-targets -- -D warnings
cargo doc -p chief-of-staff-service-registry --no-deps
```
