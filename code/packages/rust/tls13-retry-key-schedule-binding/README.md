# tls13-retry-key-schedule-binding

`tls13-retry-key-schedule-binding` joins the exact-message retry transcript and
message-bound X25519 shared secret only when both retain the same CH1,
HelloRetryRequest, CH2, and final ServerHello selection proof. It also requires
caller-owned base-secret storage to retain the transcript's exact data-driven
cipher suite before delegating to the repository-owned no-PSK key schedule.

Proof and suite checks happen before key-schedule derivation can clear or write
storage. Success retains both message-bound inputs beside the Early,
Handshake, and Master Secret view into caller-owned wipe-on-drop storage. The
package accepts no detached
message bytes, cipher-suite policy, transcript hash, or shared secret.

This pure composition does not derive handshake traffic secrets, advance the
transcript, authenticate CertificateVerify or Finished, protect records,
validate certificates, establish trust, enforce revocation, perform I/O,
authorize HTTP or OAuth, or provide TLS, HTTPS, or live-provider readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-retry-key-schedule-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-retry-key-schedule-binding --no-deps
```
