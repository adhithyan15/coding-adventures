# tls13-hkdf-label

`tls13-hkdf-label` validates and serializes the public `HkdfLabel` information
defined by RFC 8446 section 7.1. It is allocation-free, has no dependencies,
and writes only into caller-owned storage.

## Boundary

The crate binds:

- one public 16-bit requested output length;
- a caller label from 1 through 249 bytes;
- the exact six-byte `"tls13 "` namespace prefix; and
- a context from 0 through 255 bytes.

The complete encoding is at most 514 bytes. Input bounds and output capacity
are validated before output is modified. Stable errors contain categories only.

This crate takes no secret and performs no hashing, HKDF execution, traffic-key
derivation, cipher-suite selection, nonce management, AEAD operation,
certificate validation, network access, HTTP, or OAuth authorization. It does
not make TLS or concrete HTTPS available.

## Example

```rust
use tls13_hkdf_label::HkdfLabel;

let label = HkdfLabel::new(16, b"key", b"").unwrap();
let mut storage = [0u8; 13];
assert_eq!(
    label.encode(&mut storage).unwrap(),
    b"\x00\x10\x09tls13 key\x00"
);
```

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-hkdf-label
cargo clippy -p tls13-hkdf-label --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-hkdf-label --no-deps
```
