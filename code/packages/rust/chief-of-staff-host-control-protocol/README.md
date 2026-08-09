# chief-of-staff-host-control-protocol

`chief-of-staff-host-control-protocol` defines the authenticated lifecycle and
bounded data-plane conversation between the D18 orchestrator and one spawned
host. After the secure handshake, the orchestrator authenticates the exact
relevant public package key, trust class, and tier to the child. The child builds
its own verification keyring and independently verifies its signed package. The
orchestrator then authenticates pipeline-authorized signed-name-to-UUID channel
bindings and, for Level 1, bounded model settings. Only after matching those
bindings to signed policy does the child send `Ready(package_hash)`, heartbeats,
and serialized channel or
provider-neutral completion requests. The orchestrator sends exact correlated
responses or `Terminate`.

The wrapper runs over `chief-of-staff-secure-host-channel`, enforces role and
lifecycle ordering, and fails closed after malformed, unauthenticated,
wrong-direction, replayed, duplicate/missing package trust or launch bindings, or package-mismatched
peer input. Heartbeats carry no child-selected timestamp: the orchestrator attaches
its trusted monotonic receive time after authentication. The data plane permits one
request in flight, uses
strictly increasing non-zero IDs, and fails closed on skipped, duplicate,
wrong-operation, or unsolicited responses. Receive, publish, acknowledge, and
text completion records have explicit aggregate and field bounds below the
secure channel's one-megabyte frame limit.

The crate owns no clock, file descriptor, process, stream, filesystem, or network
channel-storage, model-provider, or authorization capability. A concrete process
supervisor supplies complete encrypted frames and receipt times; later adapters
execute authenticated requests against injected services.

## Validation

```sh
cargo test -p chief-of-staff-host-control-protocol -- --nocapture
cargo clippy -p chief-of-staff-host-control-protocol --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-host-control-protocol --no-deps
```
