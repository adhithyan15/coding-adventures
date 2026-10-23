# x509-rsa-sha256-reviewed-server-link

This zero-external-production-dependency crate closes one evidence-mixing gap
between target-server certificate policy and one authenticated issuer link that
terminates at a reviewed Mozilla root.

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

## Boundary

This is exact-object evidence composition plus deliberately narrow direct-link
anchor DNS policy, not complete certificate-path or TLS validation. It supports
only the repository's bounded permitted-dNSName constraint subset. It does not
apply excluded, directory-name, IP, or other constraint forms; path-length or
other anchor constraints; multi-hop construction; revocation; TLS; HTTP; OAuth
transport; or network effects.
`webpki-roots` remains reviewed public data only; no external executable TLS,
X.509, or cryptographic implementation is linked here.

The focused unit tests cover the security-sensitive exact-object predicate,
unconstrained and constrained policy branches, exact reviewed `.tr` bytes,
all-target-DNS matching, DNS-absent SANs, malformed inputs, independent decoder
budgets, and payload-free failures. The upstream evidence types intentionally
expose no forging constructors, and the repository has no direct reviewed-root-
signed end-entity fixture, so this package does not weaken those types with a
test-only construction path.
