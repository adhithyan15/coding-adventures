# tls13-server-hello-message

`tls13-server-hello-message` is a bounded structural decoder for one
already-delimited RFC 8446 `ServerHello` handshake body.

The decoder:

- enforces the TLS 1.3 legacy version (`0x0303`) and compression (`0`) fields,
- retains the exact 32-byte random, zero-to-32-byte session-id echo, and
  unclassified cipher-suite identifier,
- delegates the final length-prefixed vector to the repository-owned
  `tls13-extension-block` decoder,
- rejects duplicate extension types and malformed or trailing bytes through
  that dependency, and
- applies caller-owned ceilings to the complete body, extension payload, and
  extension count before returning borrowed values.

Unknown cipher-suite and extension identifiers remain data. The package does
not classify the `HelloRetryRequest` random, interpret extension contents,
require `supported_versions`, bind selections to a `ClientHello`, enforce the
downgrade sentinel, advance a handshake, derive keys, validate certificates,
perform I/O, or make TLS or HTTPS available.

## Example

```rust
use tls13_server_hello_message::{
    decode_tls13_server_hello, Tls13ServerHelloLimits,
};

let mut body = vec![0x03, 0x03];
body.extend_from_slice(&[0x11; 32]);
body.push(0); // legacy_session_id_echo
body.extend_from_slice(&[0x13, 0x01]);
body.push(0); // legacy_compression_method
body.extend_from_slice(&[0, 6, 0, 43, 0, 2, 0x03, 0x04]);

let limits = Tls13ServerHelloLimits::new(256, 128, 8)?;
let decoded = decode_tls13_server_hello(&body, limits)?;
assert_eq!(decoded.cipher_suite().wire_value(), 0x1301);
assert_eq!(decoded.extensions().len(), 1);
# Ok::<(), tls13_server_hello_message::Tls13ServerHelloError>(())
```

## References

- [RFC 8446 section 4.1.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.1.3)
  defines `ServerHello`, its fixed legacy fields, and its relationship to
  `HelloRetryRequest`.
- [RFC 8446 appendix B.3.1.2](https://www.rfc-editor.org/rfc/rfc8446.html#appendix-B.3.1.2)
  restates the complete wire structure.

## Development

```bash
cargo test -p tls13-server-hello-message
cargo clippy -p tls13-server-hello-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-server-hello-message --no-deps
```
