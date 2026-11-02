# tls13-supported-versions

`tls13-supported-versions` is a small allocation-free policy and framing
boundary for RFC 8446's `supported_versions` extension data.

It validates a caller-owned, preference-ordered ClientHello offer, requires
that TLS 1.3 (`0x0304`) is present, rejects duplicates, and encodes the
one-byte-length-prefixed version vector into caller-owned storage. Unknown
version identifiers remain caller-controlled data so a later policy can use
reserved or future values without changing this package.

For ServerHello and HelloRetryRequest, it accepts exactly one two-byte selected
version, verifies that the value was offered, and accepts only TLS 1.3. This is
the narrow version-negotiation policy needed by a future repository-owned TLS
1.3 client.

The package does **not** parse an extension block, decide where the extension
may appear, construct a ClientHello, process a handshake, send an alert,
provide downgrade protection, derive secrets, protect records, validate
certificates, perform I/O, or make TLS or HTTPS available.

## Example

```rust
use tls13_supported_versions::{ClientVersionOffer, TLS13_VERSION};

let configured = [TLS13_VERSION];
let offer = ClientVersionOffer::new(&configured)?;
let mut output = [0_u8; 3];

assert_eq!(offer.encode(&mut output)?, 3);
assert_eq!(output, [2, 0x03, 0x04]);
assert_eq!(offer.validate_server_selection(&[0x03, 0x04])?.wire_id(), TLS13_VERSION);
# Ok::<(), tls13_supported_versions::VersionError>(())
```

## References

- [RFC 8446 section 4.2.1](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.1)
  defines the extension's ClientHello and server response encodings and
  version-selection rules.
- [RFC 8446 appendix B.3.1.1](https://www.rfc-editor.org/rfc/rfc8446.html#appendix-B.3.1.1)
  restates the wire structures.

## Development

```bash
cargo test -p tls13-supported-versions
cargo clippy -p tls13-supported-versions --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-supported-versions --no-deps
```
