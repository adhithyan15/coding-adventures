# tls13-inner-plaintext

`tls13-inner-plaintext` is the allocation-free codec for the authenticated
cleartext inside a protected TLS 1.3 record. It composes with `tls13-record`,
which owns the outer five-byte record header and ciphertext fragment.

The wire shape is:

```text
content || content_type || zero padding
```

Decoding scans backward through the exact caller-provided cleartext until it
finds the final nonzero byte. That byte is the protected content type;
everything before it is borrowed as content, and the trailing zeros are
reported as a padding length. Callers must pass only cleartext already
authenticated by a future AEAD boundary.

```rust
use tls13_inner_plaintext::{
    decode_tls13_inner_plaintext, encode_tls13_inner_plaintext,
    Tls13InnerPlaintextPolicy,
};
use tls13_record::TlsContentType;

let policy = Tls13InnerPlaintextPolicy::new(
    &[TlsContentType::ApplicationData],
    1024,
)?;
let mut encoded = [0xa5; 32];
let used = encode_tls13_inner_plaintext(
    &policy,
    TlsContentType::ApplicationData,
    b"hello",
    3,
    &mut encoded,
)?;
let decoded = decode_tls13_inner_plaintext(&policy, &encoded[..used])?;
assert_eq!(decoded.content(), b"hello");
assert_eq!(decoded.padding_length(), 3);
# Ok::<(), tls13_inner_plaintext::InnerPlaintextError>(())
```

The policy contains an explicit, duplicate-free protected-content allowlist and
a maximum encoded length from 1 through RFC 8446's `2^14 + 1` limit. That
ceiling can carry a smaller negotiated record-size limit. Protected
ChangeCipherSpec is never accepted. Empty application data is representable;
empty handshake and alert content fail closed. Encoding validates all inputs
and output capacity before changing caller-owned storage.

This package does not perform AEAD encryption, decryption, or authentication;
manage nonces or sequence numbers; choose padding; process handshake or alert
messages; derive keys; validate certificates; perform I/O; authorize HTTP or
OAuth transport; or provide a TLS connector. Concrete HTTPS remains
unavailable.

## Validation

From `code/packages/rust`:

```sh
cargo test -p tls13-inner-plaintext
cargo clippy -p tls13-inner-plaintext --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-inner-plaintext --no-deps
```
