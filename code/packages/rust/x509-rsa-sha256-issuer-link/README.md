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

A stronger evidence-bound entry point additionally consumes one
`ValidatedCertificateRolePolicy` for the selected issuer. Before key or
signature work, it requires that evidence to retain the exact same candidate
certificate object and to have been built for the issuing-CA role. The existing
role gate still runs under this crate's caller-owned decode budget. Success
returns a distinct non-copyable value retaining both the authenticated link and
the outer role-policy evidence, so downstream code can require this stronger
composition without confusing it with the narrow result.

The strongest entry point also consumes a
`ValidatedCertificateAlgorithmPolicy` for that issuer. Before the existing
role, RSA-key, or signature work, selection, role, and algorithm evidence must
all retain the same exact candidate object. Its distinct non-copyable result
retains the authenticated role-policy link plus the certificate-bound
algorithm profile. The profile remains caller policy data: it neither routes to
nor enables this crate's fixed verifier, and its encoded public-key size does
not replace the separate RSA modulus-strength policy.

The authenticated-intermediate binding additionally consumes one complete
`ValidatedX509IntermediateCertificatePolicy`. It requires that policy and the
strongest authenticated link to retain the identical selected issuer object,
then returns a distinct non-copyable result retaining both. Byte-equal but
separately decoded issuers are rejected, so later traversal cannot combine an
authenticated link with critical-extension, algorithm, self-issued, or
validity policy for another certificate object. This proves only that the
policy-approved intermediate authenticated the retained child; it does not
authenticate the intermediate's own issuer or advance path-length state.

Critical-extension recognition remains caller-owned and is not semantic
validation of recognized extensions outside the role gate. The
role-and-algorithm result alone grants no time authority, while the
authenticated-intermediate result retains only the complete policy's validity
classification at its explicit observation. Neither result grants a complete
path, constraint, trust, identity, revocation, TLS, HTTP, OAuth transport, or
network authority.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-sha256-issuer-link --all-targets -- -D warnings
cargo doc -p x509-rsa-sha256-issuer-link --no-deps
```
