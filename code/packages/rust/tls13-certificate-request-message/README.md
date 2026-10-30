# tls13-certificate-request-message

`tls13-certificate-request-message` is an allocation-free structural decoder
for the TLS 1.3 `CertificateRequest` body. It validates one one-byte-length-
prefixed opaque request context followed by one exact nonempty generic TLS
extension block, and returns borrowed views of the complete body, context, and
ordered duplicate-free extensions.

It does not make TLS or HTTPS available. It does not process a handshake,
decide whether an empty or nonempty context is valid in the current state,
require `signature_algorithms`, decode any extension value, choose an
algorithm, access credentials, sign, update a transcript, emit alerts, derive
keys, protect records, validate certificates, check revocation, perform I/O,
or authorize HTTP or OAuth transport.

## Boundary

The caller supplies a maximum request-context length from zero through 255 and
already-validated `tls13-extension-block` limits. The decoder rejects a missing
context-length byte, a context that exceeds the caller ceiling, and a truncated
context before exposing context bytes. It then delegates every remaining byte
to the generic extension decoder and rejects a successfully decoded empty
vector, so incomplete, truncated, trailing, or empty extension encodings fail
closed under the message-specific and generic byte and entry ceilings.

Both empty and nonempty contexts are valid structural values here. Handshake
state must separately require an empty context during the main handshake and
apply uniqueness and unpredictability rules to post-handshake contexts.
Unknown extension types remain ordinary structural values. The structural
nonempty-vector rule does not prove that `signature_algorithms` or any other
specific extension is present; later policy must decide which extensions are
required or permitted and decode their contents.

The returned body borrows caller-owned bytes and allocates nothing. Diagnostics
contain stable public categories and length/count metadata only; they never
retain or render context or extension payload bytes.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-certificate-request-message
cargo clippy -p tls13-certificate-request-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-certificate-request-message --no-deps
```
