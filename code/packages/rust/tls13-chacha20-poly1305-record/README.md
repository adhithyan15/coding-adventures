# tls13-chacha20-poly1305-record

`tls13-chacha20-poly1305-record` is the allocation-free record-protection
composition for the repository-owned TLS 1.3 ChaCha20-Poly1305 route. It binds
one selected-suite-owned key/nonce reservation to the exact five-byte
`TLSCiphertext` header, the fixed-memory RFC 8439 AEAD, and a caller-authorized
`TLSInnerPlaintext` policy.

Sealing validates public policy and capacity before consuming a sequence,
encodes content plus its protected type and padding in caller scratch,
authenticates the exact outer header, appends the 16-byte tag, and clears the
scratch on every return. Opening validates one exact outer record and output
capacity before consuming a sequence. Authentication failure consumes that
sequence but releases no plaintext and leaves output unchanged. Successful
opening returns a non-cloneable borrowed view; dropping it clears the opened
prefix of caller storage.

Only `TLS_CHACHA20_POLY1305_SHA256` is admitted. AES-GCM selections fail before
nonce reservation. Callers must own distinct read and write states and choose
the correct handshake or application traffic epoch.

This crate does not derive traffic secrets, run a handshake, authenticate
`CertificateVerify` or `Finished`, validate certificates, paths, names, trust,
or revocation, update keys, read a stream, open a socket, authorize HTTP or
OAuth, provide a TLS connector, or make concrete HTTPS available.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-chacha20-poly1305-record
cargo clippy -p tls13-chacha20-poly1305-record --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-chacha20-poly1305-record --no-deps
```
