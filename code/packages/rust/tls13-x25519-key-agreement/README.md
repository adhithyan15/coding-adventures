# tls13-x25519-key-agreement

`tls13-x25519-key-agreement` is a narrow, allocation-free execution boundary
between the repository's selected TLS 1.3 named-group policy and its X25519
primitive.

```rust
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_x25519_key_agreement::derive_x25519_shared_secret;

# let private_scalar = [0x11; 32];
# let peer_public_key = [0x22; 32];
let policy = NamedGroupPolicy::new(&[NamedGroup::X25519])?;
let selected = policy.select(NamedGroup::X25519.wire_id())?;
let mut output = [0u8; 32];
let secret = derive_x25519_shared_secret(
    selected,
    &private_scalar,
    &peer_public_key,
    &mut output,
)?;
// Use `secret.as_bytes()` while the view is alive. Dropping it wipes output.
# Ok::<(), Box<dyn std::error::Error>>(())
```

The selected group must route to X25519 and retain the standard 32-byte peer
key and shared-secret lengths. P-256 is rejected before secret or peer bytes
are read. Invalid-length and low-order peer keys leave caller output unchanged.
Success returns a non-cloneable view over caller-owned output; dropping the
view wipes that exact array through the repository-owned zeroization primitive.

## Boundary

The caller still owns and must wipe the private scalar. This package does not
generate entropy or an ephemeral key, frame or bind ClientHello and ServerHello
key shares, update a transcript, advance a handshake, derive traffic secrets,
protect records, validate certificates, establish trust, check revocation,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
The existing X25519 primitive's optimized-code behavior and internal temporary
handling remain properties of that package. This crate does not establish TLS,
HTTPS, or live-provider readiness.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-x25519-key-agreement
cargo clippy -p tls13-x25519-key-agreement --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-x25519-key-agreement --no-deps
```
