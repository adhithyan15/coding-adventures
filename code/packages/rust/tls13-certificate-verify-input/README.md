# tls13-certificate-verify-input

`tls13-certificate-verify-input` is an allocation-free repository-owned
framing boundary for the content signed by a TLS 1.3 `CertificateVerify`
message. It emits exactly:

```text
64 * 0x20 || context_string || 0x00 || Transcript-Hash(Handshake Context)
```

The role selects the RFC 8446 server or client context string. The caller also
selects SHA-256 or SHA-384 metadata, which fixes the accepted transcript-hash
length at 32 or 48 bytes. The complete input and output lengths are validated
before caller-owned output changes.

This package does **not** calculate a transcript hash, encode or decode a
`CertificateVerify` handshake message, select a signature scheme, parse a key,
sign, verify a signature, process a certificate, validate a path or name,
provide trust roots or revocation, perform I/O, or make TLS or HTTPS available.
A later reviewed composition must bind the hash choice to the negotiated cipher
suite and bind this input to the selected signature scheme and peer key.

```rust
use tls13_certificate_verify_input::{
    build_certificate_verify_input, CertificateVerifyRole,
    TranscriptHashAlgorithm, MAXIMUM_INPUT_LENGTH,
};

let transcript_hash = [0x42; 32];
let mut output = [0u8; MAXIMUM_INPUT_LENGTH];
let input = build_certificate_verify_input(
    CertificateVerifyRole::Server,
    TranscriptHashAlgorithm::Sha256,
    &transcript_hash,
    &mut output,
)?;

assert_eq!(input.as_bytes().len(), 130);
# Ok::<(), tls13_certificate_verify_input::InputError>(())
```

## References

- [RFC 8446 section 4.4.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.4.3)
  defines the `CertificateVerify` signed content and role-specific context
  strings.

## Development

```bash
cargo test -p tls13-certificate-verify-input
cargo clippy -p tls13-certificate-verify-input --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-certificate-verify-input --no-deps
```
