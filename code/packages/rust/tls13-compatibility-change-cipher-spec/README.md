# tls13-compatibility-change-cipher-spec

`tls13-compatibility-change-cipher-spec` is the allocation-free RFC 8446
middlebox-compatibility body layer for a future repository-owned TLS 1.3
client. It accepts exactly the single canonical octet `0x01`, rejects every
other value and body length, and returns an unforgeable typed value that
retains no caller bytes.

Encoding writes the canonical body into caller-owned output only after
capacity validation and leaves extra storage untouched. Stable failures expose
only categories, never input values or bytes.

This package receives content already delimited from an unprotected
ChangeCipherSpec record. It does not authorize the record in a handshake
state, decide whether to send or silently discard it, update a transcript,
accept protected ChangeCipherSpec content, sequence handshake messages, send
an alert, derive keys, protect records, perform I/O, authorize HTTP or OAuth
traffic, or provide concrete HTTPS or live-provider readiness.

## Example

```rust
use tls13_compatibility_change_cipher_spec::{
    decode_compatibility_change_cipher_spec,
    encode_compatibility_change_cipher_spec,
};

let message = decode_compatibility_change_cipher_spec(&[1])?;
let mut output = [0_u8; 1];
let used = encode_compatibility_change_cipher_spec(message, &mut output)?;
assert_eq!(&output[..used], &[1]);
# Ok::<(), tls13_compatibility_change_cipher_spec::CompatibilityChangeCipherSpecError>(())
```

## Development

```bash
bash BUILD
cargo clippy -p tls13-compatibility-change-cipher-spec --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-compatibility-change-cipher-spec --no-deps
```
