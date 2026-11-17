# tls13-x25519-key-share-binding

`tls13-x25519-key-share-binding` is a narrow, allocation-free composition
boundary between retained TLS 1.3 key shares and repository-owned X25519
execution.

It accepts one decoded ServerHello key share already bound to an exact retained
ClientHello offer. The selected group must route to X25519 before the caller's
private scalar is read. The package derives that scalar's public key and
compares it with the retained X25519 client share through the repository-owned
compiler-preserved full-scan fixed-length comparison. Only a match can pass the
exact decoded server public share to the selected-group-bound X25519 execution
boundary.

Mismatch, unsupported-group, public-key derivation, and downstream key-
agreement failures leave caller-owned shared-secret output unchanged. Success
returns the existing non-cloneable shared-secret view; dropping it wipes the
exact borrowed output.

## Boundary

The caller still owns and must wipe the private scalar. This package does not
generate entropy or an ephemeral key, prove that the retained offer was sent,
bind either hello to a transcript, advance a handshake, derive traffic secrets,
protect records, validate certificates, establish trust, check revocation,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
The comparison inherits its repository package's compiler-observability
guarantee and adds no hardware side-channel claim. The existing X25519
primitive's optimized-code behavior and internal temporary handling remain
properties of that package. This crate does not establish TLS, HTTPS, or live-
provider readiness.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-x25519-key-share-binding
cargo clippy -p tls13-x25519-key-share-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-x25519-key-share-binding --no-deps
```
