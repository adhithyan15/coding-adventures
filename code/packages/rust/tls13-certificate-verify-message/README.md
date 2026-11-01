# tls13-certificate-verify-message

`tls13-certificate-verify-message` is an allocation-free repository-owned codec
for the TLS 1.3 `CertificateVerify` message body:

```text
uint16 SignatureScheme || uint16 signature_length || signature
```

Decoding requires one complete body and rejects truncation, a declared length
that is shorter or longer than the remaining bytes, and trailing bytes.
Encoding validates the complete body and caller-owned output capacity before
changing output. The RFC vector syntax permits signature lengths from zero
through 65,535 bytes; accepting an empty value structurally does not make it a
valid cryptographic signature.

The two-byte algorithm identifier remains uninterpreted wire data. A later
policy boundary must recognize and authorize it, and a later verifier must
reject invalid or empty signatures. This package does **not** frame the outer
handshake header, select a signature scheme, construct signed content, hash,
sign, verify, parse keys or certificates, validate paths or names, provide
trust roots or revocation, perform I/O, or make TLS or HTTPS available.

```rust
use tls13_certificate_verify_message::CertificateVerifyMessage;

let body = [0x08, 0x04, 0x00, 0x03, 0xaa, 0xbb, 0xcc];
let message = CertificateVerifyMessage::decode(&body)?;
assert_eq!(message.algorithm_wire_id(), 0x0804);
assert_eq!(message.signature(), &[0xaa, 0xbb, 0xcc]);
# Ok::<(), tls13_certificate_verify_message::MessageError>(())
```

## References

- [RFC 8446 section 4.4.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.4.3)
  defines the `CertificateVerify` message body.

## Development

```bash
cargo test -p tls13-certificate-verify-message
cargo clippy -p tls13-certificate-verify-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-certificate-verify-message --no-deps
```
