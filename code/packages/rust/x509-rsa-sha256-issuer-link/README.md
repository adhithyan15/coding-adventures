# x509-rsa-sha256-issuer-link

A bounded repository-owned composition for one directed RSA PKCS #1 v1.5
SHA-256 certificate issuer link.

For one exact child and one exact candidate issuer, the crate:

1. invokes caller-owned distinguished-name policy exactly once with the child
   issuer followed by the candidate issuer subject;
2. requires the candidate issuer's critical `cA = TRUE` Basic Constraints and
   `keyCertSign` Key Usage through the existing role gate;
3. parses and validates that exact candidate's RSA public key under
   caller-supplied modulus and exponent policy; and
4. verifies the child's preserved `TBSCertificate` bytes and exact signature
   with that key.

Success returns non-copyable borrowed evidence that binds the exact child,
candidate issuer, and validated issuer key. It does not process the
candidate's other critical extensions, check validity, enforce path length or
name constraints, construct a path, select or recognize a trust anchor, check
revocation, or provide TLS, HTTP, OAuth transport, or network authority.

An evidence-bound entry point also consumes one
`SelectedX509IssuerCandidate`. It derives both certificate references from that
non-copyable selection evidence and applies the CA-role, RSA-key, and signature
checks without invoking Name policy again. This prevents a caller from
selecting one candidate and accidentally authenticating another, while still
granting no complete-path or trust authority.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha256-issuer-link --all-targets -- -D warnings
cargo doc -p x509-rsa-sha256-issuer-link --no-deps
```
