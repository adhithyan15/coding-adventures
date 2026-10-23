# x509-rsa-sha256-reviewed-server-link

This zero-external-production-dependency crate closes one evidence-mixing gap
between target-server certificate policy and authenticated RSA SHA-256 evidence
that terminates at a reviewed Mozilla root.

`bind_x509_rsa_sha256_reviewed_server_link` consumes:

- `ValidatedX509ServerCertificatePolicy`, which retains the exact target after
  caller-owned critical-extension and algorithm policy, end-entity role,
  SAN-only identity, explicit-time validity, and exact-purpose checks; and
- `BoundX509RsaSha256ReviewedAnchorLink`, which retains one RSA SHA-256
  child-to-issuer authentication and exact reviewed-root selection.

The binder succeeds only when both non-copyable inputs retain the same decoded
child certificate object. Equal bytes decoded independently are insufficient.
The distinct non-copyable result retains both complete inputs for later path
policy.

`bind_x509_rsa_sha256_reviewed_intermediate_server_path` instead consumes:

- `ValidatedX509ServerCertificatePolicy`, retaining the same complete target
  policy as the direct-link composition; and
- `BoundX509RsaSha256ReviewedIntermediatePath`, retaining authenticated
  target-to-intermediate and intermediate-to-reviewed-root links, complete
  intermediate policy, advanced path-length state, and exact root selection.

It succeeds only when the server-policy certificate and the child authenticated
by the first path step are the same decoded object. Equal bytes decoded
independently are insufficient. Its distinct non-copyable result retains both
complete inputs and does not discard the path-length transition.

`validate_x509_rsa_sha256_reviewed_server_anchor_policy` then consumes that
complete direct-link evidence and checks the selected snapshot entry itself.
Entries without out-of-band name constraints produce a stronger non-copyable
anchor-policy result without DNS evidence. For a constrained entry, the policy
decodes the exact selected-entry bytes and re-decodes Subject Alternative Name
from the exact target certificate retained by the server-policy evidence. It
requires every target dNSName to satisfy at least one permitted DNS subtree and
retains optional payload-free count evidence. Callers supply separate ASN.1
limits for the two parses and cannot inject another anchor, certificate, or SAN
collection.

`validate_x509_rsa_sha256_reviewed_intermediate_server_anchor_policy` applies
the same selected-entry policy to the complete two-link server path. It takes
the target certificate only from the retained server policy and the constraint
bytes only from the root selected by the retained reviewed parent link. Its
distinct non-copyable result keeps the whole target-to-intermediate-to-root
proof, including intermediate policy and path-length state, plus the optional
payload-free DNS count evidence.

`validate_x509_rsa_sha256_reviewed_intermediate_server_name_constraints_policy`
then consumes that complete anchor-policy result. It locates NameConstraints
only on the exact intermediate retained by the authenticated path. An absent
extension preserves the prior proof without new DNS evidence. A present
extension must be critical and must be one exact DER NameConstraints SEQUENCE
inside the repository's permitted-and-excluded-dNSName subset. The policy
re-decodes Subject Alternative Name from the exact retained target, validates
every constraint, rejects excluded names and any wildcard overlapping an
excluded subtree before applying the permitted containment rule, and retains
the complete prior proof plus optional payload-free intermediate DNS evidence.

## Boundary

This is exact-object evidence composition plus deliberately narrow selected-
anchor and intermediate DNS policy for direct and exactly one-intermediate RSA
SHA-256 paths, not complete certificate-path or TLS validation. The reviewed
snapshot input remains permitted-dNSName-only; exact critical intermediate
extensions support bounded permitted and excluded dNSName subtrees. The crate
does not apply directory-name, IP, or other constraint forms; other anchor
constraints; path search or generalized construction;
revocation; TLS; HTTP; OAuth transport; or network effects.
`webpki-roots` remains reviewed public data only; no external executable TLS,
X.509, or cryptographic implementation is linked here.

The focused unit and integration tests cover the security-sensitive
exact-object predicate, the public two-link target binding with a static public
Let's Encrypt precertificate, R13 intermediate, and reviewed ISRG Root X1, the
two-link anchor-policy and intermediate-policy retention, byte-equal separately decoded target
rejection, complete-input getters,
unconstrained and constrained policy branches, exact reviewed `.tr` bytes,
all-target-DNS matching, DNS-absent SANs, criticality, exclusion precedence,
unsupported non-DNS forms, malformed inputs, independent decoder
budgets, and payload-free failures. The static integration fixtures record
their public sources and SHA-256 digests; tests perform no network effects. The
upstream evidence types intentionally expose no forging constructors, so this
package does not weaken those types with a test-only construction path.
