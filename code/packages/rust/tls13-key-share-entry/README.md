# tls13-key-share-entry

`tls13-key-share-entry` is an allocation-free structural codec for the TLS 1.3
`KeyShareEntry` shared by ClientHello and ServerHello key-share extension data.

A caller supplies one exact two-byte named-group identifier, a non-empty
borrowed key-exchange payload, and an explicit maximum payload length. Unknown
or reserved group identifiers remain exact data for a later group policy.
Encoding checks the entire output capacity before writing. Prefix decoding
returns one complete borrowed entry and the unconsumed input, and rejects an
over-policy declared length before requiring that payload to be buffered.

This package does **not** construct a key-share vector, authorize a group,
enforce group-specific key encoding, generate ephemeral secrets, perform key
agreement, validate peer public keys, derive a shared secret, advance a TLS
handshake, or make TLS or HTTPS available.

## Example

```rust
use tls13_key_share_entry::{KeyShareEntry, KeyShareLimits};

let limits = KeyShareLimits::new(32)?;
let public_bytes = [0x5a; 32];
let entry = KeyShareEntry::new(0x001d, &public_bytes, limits)?;
let mut encoded = [0_u8; 36];

assert_eq!(entry.encode(&mut encoded)?, 36);
let (decoded, remainder) = KeyShareEntry::decode_prefix(&encoded, limits)?;
assert_eq!(decoded.group(), 0x001d);
assert_eq!(decoded.key_exchange(), &public_bytes);
assert!(remainder.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The numeric group in this example demonstrates wire preservation only; this
package does not authorize that group or any associated implementation.

## Reference

- [RFC 8446 section 4.2.8](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.8)
  defines `KeyShareEntry` and its use in ClientHello and ServerHello.

## Development

```bash
cargo test -p tls13-key-share-entry
cargo clippy -p tls13-key-share-entry --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-key-share-entry --no-deps
```
