# x509-rsa-sha512-issuer-link

Bounded repository-owned composition for one directed RSA PKCS #1 v1.5
SHA-512 certificate issuer link.

For one exact child and one exact candidate issuer, the crate:

1. invokes caller-owned distinguished-name policy exactly once with the child
   issuer followed by the candidate issuer subject;
2. requires the candidate issuer's algorithm-independent issuing-CA role;
3. parses and validates that exact candidate's RSA public key under
   caller-supplied modulus and exponent policy; and
4. verifies the child's preserved `TBSCertificate` bytes and exact
   `sha512WithRSAEncryption` signature with that key.

Success returns non-copyable borrowed evidence binding the exact child,
candidate issuer, and validated issuer key. It does not route algorithms,
process all critical extensions, check issuer validity, enforce path length or
name constraints, construct or validate a complete path, select a reviewed
trust anchor, check revocation, perform TLS or HTTP, emit audit or trace
events, or make OAuth HTTPS/live-provider use ready.

A construction-bound entry point consumes one
`ConstructedX509CertificatePathAdjacentLink`. It derives the target child at
depth zero or the preceding selected candidate at later depths, authenticates
that exact child against the exact projected issuer without rerunning Name
policy, and returns non-copyable evidence retaining the projection and
validated issuer key. This preserves one construction step's order without
authenticating another link, applying complete issuer policy, validating a
path, or establishing trust.

A stronger construction-bound entry point accepts complete intermediate policy
only for the exact projected non-terminal issuer. It rejects a terminal issuer
or policy for any other decoded object before RSA work, then retains the
authenticated adjacency and complete policy together. Its bounded path-length
follow-up derives the transition only from that retained policy and keeps the
ordered proof with the advanced state. The caller still supplies the initial
state; this composition does not authenticate another adjacency, prove prior
traversal, validate a complete path, or establish trust.

The stronger policy-link entry point consumes certificate-wide critical-
extension and issuing-CA role evidence plus certificate-bound, data-driven RSA
algorithm/key evidence. It rejects evidence retained from any other decoded
certificate object before invoking Name policy or RSA verification, then
retains the exact evidence in its result. The entry point fixes SHA-512 in its
type and never treats algorithm-policy data as verifier routing authority.

The complete-intermediate-policy binder requires the authenticated candidate
issuer and the complete intermediate policy to retain the exact same decoded
certificate object. Its non-copyable result retains the authenticated link and
the complete certificate-wide critical-extension, issuing-CA role,
data-driven algorithm/key, self-issued-classification, and explicit-time
validity evidence. A bounded follow-up consumes that exact proof and one
caller-owned path-length state, then retains the proof together with the state
advanced by the repository-owned RFC 5280 transition. It derives issuing-CA
role, self-issued classification, and any `pathLenConstraint` only from the
retained intermediate policy. It does not authenticate another link, establish
path order, evaluate NameConstraints, construct a complete path, select trust,
check revocation, perform TLS or HTTP, make an OAuth request, emit audit or
trace events, or grant provider or network authority.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha512-issuer-link --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-sha512-issuer-link --no-deps
```
