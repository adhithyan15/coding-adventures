# tls13-retry-handshake-traffic-key-material-binding

`tls13-retry-handshake-traffic-key-material-binding` accepts only the existing
exact-message `BoundRetryHandshakeTrafficSecrets` evidence, one explicit
client-or-server sender role, and caller-owned traffic-key-material storage. It
imports the retained sender's exact handshake traffic secret into a fixed
wipe-on-drop PRK and derives the selected suite's traffic key and static IV.

Caller-owned output storage must retain the exact selected suite before any
mutation. Success returns a non-cloneable wrapper retaining the complete CH1,
HelloRetryRequest, CH2, final ServerHello, base-schedule, and handshake-secret
proof beside the sender role and a borrowed key-material view into caller-owned
fixed wipe-on-drop storage. Callers cannot supply a detached secret, suite,
hash, output length, label, or context.

The complete traffic-secret wrapper can instead be consumed into a single-use
record-state issuer. That issuer permits one successful client issuance and
one successful server issuance. Repeating a sender is rejected before output
storage is touched. Each success derives the retained sender's key material,
moves it into fresh sequence-zero nonce state, marks that sender issued, and
returns a non-cloneable wrapper that borrows the issuer. Callers can borrow or
mutably borrow the state through that wrapper, but cannot detach it from the
complete retry proof or create a second sequence-zero state for the same
sender through this boundary.

This package derives one direction's handshake key and IV and can create the
matching record state. It does not advance the transcript, authenticate
CertificateVerify or Finished, invoke an AEAD, protect records, validate
certificates, establish trust, verify hostnames, enforce revocation, perform
I/O, authorize HTTP or OAuth, or provide TLS, HTTPS, or live-provider readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-retry-handshake-traffic-key-material-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-retry-handshake-traffic-key-material-binding --no-deps
```
