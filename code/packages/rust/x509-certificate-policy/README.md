# x509-certificate-policy

`x509-certificate-policy` supplies small RFC 5280 policy gates above the
repository-owned structural certificate decoder. Its first gate rejects the
first critical extension whose OID is not recognized by a caller-supplied
policy.

## Security boundary

The recognizer is invoked only for critical extensions, in certificate wire
order. An absent extension sequence and unrecognized non-critical extensions
are accepted. A rejected critical extension fails closed with only its
zero-based sequence index; the error retains no OID or certificate bytes.

Recognition is not semantic validation. The caller must separately decode and
enforce every recognized extension according to the certificate's role. This
crate embeds no extension registry and grants no certificate role or trust.
It does not validate Basic Constraints, Key Usage, Extended Key Usage, names,
paths, signatures, algorithms, keys, validity against a clock, revocation,
hostnames, trust roots, TLS, HTTP, or OAuth transport.

## Verification

```bash
bash BUILD
cargo clippy -p x509-certificate-policy --all-targets -- -D warnings
cargo doc -p x509-certificate-policy --no-deps
```
