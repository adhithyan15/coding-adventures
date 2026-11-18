# tls13-handshake-traffic-secrets

`tls13-handshake-traffic-secrets` derives the client and server TLS 1.3
handshake traffic secrets from one existing suite-selected `BaseSecrets` view
and one repository-owned `TranscriptHash`. Caller-owned storage must retain the
same selected suite, and the snapshot algorithm must match that suite's HKDF
hash.

The implementation imports the Handshake Secret into a repository-owned fixed
wipe-on-drop PRK, applies only the fixed `"c hs traffic"` and
`"s hs traffic"` labels, stages both Hash-length results in wipe-on-drop
owners, and publishes them to caller-owned wipe-on-drop storage only after both
derivations succeed. The returned borrowed view is non-cloneable and reports no
secret bytes through Debug or errors.

This package accepts an already-computed transcript snapshot. It does not prove
which messages produced that snapshot, sequence a handshake, bind outputs to an
exact retry exchange, derive traffic keys or IVs, authenticate Finished,
protect records, validate certificates, establish trust, enforce revocation,
perform I/O, authorize HTTP or OAuth, or provide TLS, HTTPS, or live-provider
readiness.

## Verification

```bash
bash BUILD
cargo clippy -p tls13-handshake-traffic-secrets --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-handshake-traffic-secrets --no-deps
```
