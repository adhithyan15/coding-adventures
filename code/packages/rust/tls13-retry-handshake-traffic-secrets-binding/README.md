# tls13-retry-handshake-traffic-secrets-binding

`tls13-retry-handshake-traffic-secrets-binding` accepts only the existing
exact-message `BoundRetryBaseSecrets` evidence. It snapshots the retained CH1,
HelloRetryRequest, CH2, and final ServerHello transcript, then derives client
and server handshake traffic secrets from the base schedule attached to those
same messages.

Caller-owned output storage must retain the exact selected suite before any
mutation. Success returns a non-cloneable wrapper retaining the complete retry
base-secret proof and a borrowed view into fixed wipe-on-drop traffic-secret
storage. A sealed internal evidence boundary prevents detached message, suite,
transcript-hash, or base-secret substitution.

This package does not advance the transcript beyond ServerHello, derive
traffic keys or IVs, authenticate CertificateVerify or Finished, protect
records, validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth, or provide TLS, HTTPS, or live-provider
readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-retry-handshake-traffic-secrets-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-retry-handshake-traffic-secrets-binding --no-deps
```
