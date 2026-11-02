# x509-rsa-algorithm-policy

`x509-rsa-algorithm-policy` closes the evidence gap between the repository's
data-driven certificate algorithm profiles and its exact RSA public-key
validator. It consumes non-copyable algorithm-policy evidence for one decoded
certificate, derives the SubjectPublicKeyInfo only from that retained object,
and validates the nested PKCS #1 modulus and exponent under caller-owned RSA
strength policy and ASN.1 limits.

Success retains the original algorithm-policy evidence and the borrowed
validated RSA key. Callers cannot substitute a byte-equal independently
decoded certificate or a detached key. The selected profile remains policy
data only: this package does not choose or invoke a signature verifier.

The composition parses public certificate data and performs no credential
access, signing, storage, clock, I/O, audit, provider, or network effect. It
does not verify a certificate signature, assign a certificate role, construct
or validate a path, select trust, check names or revocation, perform TLS or
HTTP, authorize OAuth transport, or make concrete HTTPS available.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-algorithm-policy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-algorithm-policy --no-deps
```
