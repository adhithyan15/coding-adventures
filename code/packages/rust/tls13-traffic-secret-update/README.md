# tls13-traffic-secret-update

`tls13-traffic-secret-update` is the repository-owned RFC 8446 section 7.2
derivation from one TLS 1.3 application traffic secret to its successor:

```text
application_traffic_secret_N+1 =
    HKDF-Expand-Label(application_traffic_secret_N,
                      "traffic upd", "", Hash.length)
```

The protocol definition is in
[RFC 8446 section 7.2](https://www.rfc-editor.org/rfc/rfc8446.html#section-7.2).

The current `HkdfPrk` carries the data-driven SHA-256 or SHA-384 choice. The
crate fixes the label and empty context internally, requires caller-owned
output to equal that hash's exact length, and delegates to the repository-owned
fixed-memory `HKDF-Expand-Label` boundary. Public validation happens before
output changes. Successful secret bytes remain in caller-owned storage.

## Boundary

This crate derives bytes only. It does not authorize a `KeyUpdate`, sequence
post-handshake messages, retain secrets, import the result into a new PRK,
install a key, reset a record sequence, enforce old-key protection, derive a
traffic key or IV, send a reciprocal update, protect a record, emit an audit
event, perform I/O, establish TLS or HTTPS, or authorize OAuth transport.

The caller must use wipe-on-drop output storage, import success into a fresh
`HkdfPrk`, and atomically coordinate secret installation with new traffic-key,
IV, and record-sequence state. Those stateful operations remain separate
reviewed composition work.

## Example

```rust
use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use coding_adventures_zeroize::Zeroizing;
use tls13_traffic_secret_update::derive_next_traffic_secret;

let current = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32])?;
let mut next = Zeroizing::new([0u8; 32]);
derive_next_traffic_secret(&current, &mut next[..])?;
let next_owner = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &next[..])?;
assert_eq!(next_owner.algorithm(), FixedHkdfHash::Sha256);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-traffic-secret-update --all-targets
cargo clippy -p tls13-traffic-secret-update --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-traffic-secret-update --no-deps
```
