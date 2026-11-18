# tls13-retry-transcript-binding

`tls13-retry-transcript-binding` constructs one TLS 1.3 retry transcript from
the exact four-message selection proof retained by
`tls13-server-hello-retry-selection`.

It frames each retained handshake body with the canonical handshake type and
three-byte length, hashes ClientHello1 through both supported transcript
algorithms, selects the policy-authorized cipher suite carried by the retained
HelloRetryRequest proof, applies the repository-owned one-shot synthetic
`message_hash` rewrite, and then hashes the exact HelloRetryRequest,
ClientHello2, and final ServerHello bodies in order. The returned non-copyable
value keeps the selected transcript attached to the exact message proof.

This pure package does not validate every permitted ClientHello2 change,
sequence a connection state machine, combine the transcript with key agreement,
derive secrets, authenticate Finished, protect records, validate certificates,
establish trust, enforce revocation, perform I/O, authorize HTTP or OAuth, or
provide TLS, HTTPS, or live-provider readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-retry-transcript-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-retry-transcript-binding --no-deps
```
