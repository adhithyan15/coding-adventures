# tls13-finished-transcript-binding

`tls13-finished-transcript-binding` closes the detached transcript-hash input
at the repository-owned TLS 1.3 Finished verifier. It accepts one immutable
`SelectedTls13Transcript`, one already-derived `FinishedKey`, and exact peer
verify-data. The selected suites must match exactly before HMAC work. The
transcript is snapshot internally, and success returns non-copyable evidence
retaining that exact transcript, exact snapshot, and `VerifiedFinished`
together.

The retained immutable transcript borrow prevents later mutation while the
proof is live. Stable failures retain no transcript bytes, digest, verify-data,
suite, role, length, or offset. Both SHA-256 and SHA-384 policy routes are
covered, and a same-hash different-suite substitution fails before peer-length
processing.

This package does not prove which ordered handshake encodings entered the
transcript, choose the correct client/server BaseKey or sender role, connect a
CertificateVerify transcript to a later Finished transcript, sequence a
handshake, authenticate certificates, establish trust, enforce revocation,
protect records, perform I/O, authorize HTTP or OAuth transport, or open a
network connection. It accesses no credential and causes no signing, provider,
trace, storage, clock, network, or other external effect. Concrete HTTPS and
live-provider readiness remain unavailable.

## Development

From `code/packages/rust`:

```bash
cargo test -p tls13-finished-transcript-binding
cargo clippy -p tls13-finished-transcript-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-finished-transcript-binding --no-deps
```
