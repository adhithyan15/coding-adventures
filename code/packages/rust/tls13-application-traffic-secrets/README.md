# tls13-application-traffic-secrets

`tls13-application-traffic-secrets` derives the client and server TLS 1.3
application traffic secrets from one existing suite-selected `BaseSecrets` view
and one repository-owned `TranscriptHash`. Caller-owned storage must retain the
same selected suite, and the snapshot algorithm must match that suite's HKDF
hash.

The implementation imports the Master Secret into a repository-owned fixed
wipe-on-drop PRK, applies only the fixed `"c ap traffic"` and
`"s ap traffic"` labels, stages both Hash-length results in wipe-on-drop
owners, and publishes them to caller-owned wipe-on-drop storage only after both
derivations succeed. The returned borrowed view is non-cloneable and reports no
secret bytes through Debug or errors.

This package accepts an already-computed transcript snapshot. It does not prove
that the snapshot ends at the authenticated server Finished, append that
Finished encoding, sequence a handshake, bind outputs to an exact retry
exchange, derive traffic keys or IVs, install later record epochs, protect
records, validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth, or provide TLS, HTTPS, or live-provider readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-application-traffic-secrets --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-application-traffic-secrets --no-deps
```
