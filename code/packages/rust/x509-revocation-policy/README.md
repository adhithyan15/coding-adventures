# x509-revocation-policy

`x509-revocation-policy` is a repository-owned, allocation-free policy gate for
one explicit caller-supplied certificate-status observation. It accepts only a
`Good` status with both `thisUpdate` and `nextUpdate`, requires an ordered
inclusive update interval, and requires one explicit validated caller-owned
time to fall inside that interval. Missing, revoked, unknown, future, inverted,
and stale observations fail with stable payload-free categories.

## Security boundary

The observation is policy input, not authenticated evidence. This package does
not parse or verify OCSP responses or CRLs, authorize responders or
distribution points, bind status to a certificate or constructed path, read a
clock, apply tolerance, cache or fetch revocation data, perform network effects,
or grant certificate trust, TLS, HTTP, or OAuth transport authority. Its
non-copyable success result retains the exact observation and explicit time and
proves only that those caller-owned values passed this strict policy gate.

## Verification

```bash
bash BUILD
cargo clippy -p x509-revocation-policy --all-targets -- -D warnings
cargo doc -p x509-revocation-policy --no-deps
```
