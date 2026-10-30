# tls13-client-hello-message

`tls13-client-hello-message` is an allocation-free decoder for one exact RFC
8446 section 4.1.2 TLS 1.3 `ClientHello` handshake body.

It enforces the fixed `0x0303` legacy version, borrows the exact 32-byte random
and bounded legacy session ID, validates a non-empty even cipher-suite vector
under an explicit caller count ceiling, requires the single null legacy
compression method, and delegates the mandatory extension vector to
`tls13-extension-block` under explicit byte and count limits. Unknown suite and
extension identifiers remain ordered data for later policy. Stable errors and
diagnostics disclose none of the random, session, suite, or extension bytes.

This package does not classify first/retry ClientHellos, generate entropy,
authorize session IDs or cipher suites, require or interpret individual
extensions, apply extension-context policy, bind later selections to this
offer, process a transcript, derive keys, invoke cryptography, validate
certificates, establish trust, enforce revocation, perform I/O, provide
HTTP/OAuth transport, or make concrete HTTPS or live-provider use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-client-hello-message --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-client-hello-message --no-deps
```
