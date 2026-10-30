# tls13-key-update-message

`tls13-key-update-message` is the allocation-free RFC 8446 section 4.6.3
message-body layer for a future repository-owned TLS 1.3 client. It accepts
exactly one byte, rejects every value other than `update_not_requested` and
`update_requested`, and exposes the required reciprocal request as typed data.

Encoding writes into caller-owned output only after capacity validation and
leaves extra storage untouched. Stable failures retain no request values or
input bytes.

This package does not decide whether the peer has sent `Finished`, sequence
post-handshake messages, coalesce response requests, rotate traffic secrets,
enforce old-key record protection or message alignment, send alerts, perform
I/O, authorize HTTP or OAuth traffic, or provide concrete HTTPS or
live-provider readiness.

## Example

```rust
use tls13_key_update_message::{decode_key_update, KeyUpdateRequest};

let request = decode_key_update(&[1])?;
assert_eq!(request, KeyUpdateRequest::UpdateRequested);
# Ok::<(), tls13_key_update_message::KeyUpdateError>(())
```

## Development

```bash
bash BUILD
cargo clippy -p tls13-key-update-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-key-update-message --no-deps
```
