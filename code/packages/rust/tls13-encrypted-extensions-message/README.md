# tls13-encrypted-extensions-message

`tls13-encrypted-extensions-message` is an allocation-free repository-owned
decoder for the exact RFC 8446 section 4.3.1 `EncryptedExtensions` handshake
body.

It delegates the complete two-byte-length-prefixed vector to
`tls13-extension-block` under explicit caller-selected byte and entry limits.
The returned non-copyable value retains both the exact borrowed body and the
structurally decoded, wire-ordered extension block. Empty extension vectors are
valid, while malformed lengths, truncation, duplicate types, and resource
limit failures keep the existing payload-free error categories.

This package embeds no extension registry and applies no message-context or
request/response policy. It does not decode extension data, sequence a TLS
handshake, update a transcript, validate certificates, establish trust, check
revocation, protect records, perform I/O, provide HTTP/OAuth transport, or make
HTTPS or live-provider use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-encrypted-extensions-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-encrypted-extensions-message --no-deps
```
