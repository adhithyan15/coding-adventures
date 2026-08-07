# chief-of-staff-secure-host-channel

`chief-of-staff-secure-host-channel` is the transport-independent security
kernel for D18 Chief host processes. It turns an unlocked orchestrator identity
into a fresh per-spawn X3DH offer, lets the child answer with an authenticated
hello, and wraps subsequent Vault Double Ratchet messages in strict bounded
`D18F` records.

The crate owns no file descriptors and performs no process, filesystem, or
network I/O. A supervisor carries `BootstrapOffer`, `ClientHello`, and encrypted
frame bytes over an inherited pipe, stdio, a Unix socket, or a Windows named
pipe. Host ID, UUID-v7 session ID, direction, and exact sequence are bound into
AEAD additional authenticated data on every frame.

Malformed structures are rejected before ratchet state advances. An AEAD
authentication failure permanently closes the channel because the underlying
ratchet may consume receive state while authenticating.

## Validation

```sh
cargo test -p chief-of-staff-secure-host-channel -- --nocapture
cargo clippy -p chief-of-staff-secure-host-channel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p chief-of-staff-secure-host-channel --no-deps
```
