# tls13-alert-message

`tls13-alert-message` is the allocation-free RFC 8446 section 6 message layer
for a future repository-owned TLS 1.3 client. It decodes an exact two-octet
alert and derives the TLS 1.3 disposition from the description rather than the
untrusted legacy severity byte.

The decoder recognizes every standard description and preserves unknown
values, which TLS 1.3 requires consumers to treat as errors. Only
`close_notify` and `user_canceled` produce closure dispositions. The encoder
admits no unknown descriptions and requires `fatal` for every standard error
alert. It preflights all policy and capacity failures before changing
caller-owned output.

This package receives already authenticated alert content. It does not own
alert sequencing or connection shutdown, remember closure state, suppress
later data, wipe connection secrets, log or audit, choose an alert for another
failure, protect records, perform I/O, close a transport, authorize HTTP or
OAuth traffic, or provide concrete HTTPS or live-provider readiness.

## Example

```rust
use tls13_alert_message::{decode_alert_message, AlertDisposition};

let alert = decode_alert_message(&[255, 0])?;
assert_eq!(alert.disposition(), AlertDisposition::CloseNotify);
# Ok::<(), tls13_alert_message::AlertError>(())
```

## Development

```bash
bash BUILD
cargo clippy -p tls13-alert-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-alert-message --no-deps
```
