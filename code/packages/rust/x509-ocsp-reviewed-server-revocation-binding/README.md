# x509-ocsp-reviewed-server-revocation-binding

`x509-ocsp-reviewed-server-revocation-binding` closes one exact-object gap
between the repository-owned OCSP revocation path and the strongest existing
constructed reviewed-server path evidence. It consumes:

- `AuthenticatedX509OcspRsaSha256TargetRevocation`, retaining the exact target,
  issuer, delegated responder, signed single-status response, freshness
  interval, observation time, and both target/responder issuer links; and
- `ValidatedX509RsaSha256ConstructedReviewedIntermediateServerNameConstraintsPolicy`,
  retaining exact target-server policy, constructed two-link RSA SHA-256 path,
  path-length state, selected Mozilla-reviewed root, selected-anchor DNS
  policy, and supported intermediate DNS-constraint policy.

Binding succeeds only when both inputs retain the same exact decoded target
and intermediate issuer objects and both exact path certificates are valid at
the OCSP proof's retained observation time. Byte-equal independent decodes and
cross-time validity evidence fail closed. The non-copyable result retains both
complete inputs plus fresh exact-time target and issuer validity tokens.
Failures expose only stable mismatch or target/issuer validity categories and
retain no certificate, response, key, signature, status, hostname, or time
payloads.

The parallel
`bind_x509_ocsp_revocation_to_routed_reviewed_server_path` boundary consumes
the newer
`ValidatedX509RsaConstructedReviewedIntermediateServerNameConstraintsPolicy`.
That path retains certificate-bound algorithm and issuer-role policy plus the
selected repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512
verifier for each exact adjacency. The binder applies the same exact target-
then-intermediate object identity checks and the same target-then-intermediate
validity checks at the OCSP observation time. Its distinct non-copyable result
retains both complete inputs and the two fresh validity tokens. It does not yet
enter the fixed-SHA-256 responder-path projection or its responder DNS and
no-check policies.

The additive
`bind_x509_ocsp_routed_response_to_routed_reviewed_server_path` boundary also
consumes the routed reviewed-server path, but joins it to
`AuthenticatedX509OcspRsaTargetRevocation`. That proof retains data-driven RSA
SHA-256, SHA-384, or SHA-512 selection for the OCSP response signature. Its
delegated-responder and target certificate issuer links remain fixed RSA
SHA-256 proofs, while each server-path adjacency is independently routed. The
distinct non-copyable result preserves both complete inputs and fresh exact-
observation validity for the same exact target and intermediate objects. It
does not route the OCSP certificate links or enter responder-path policy.

The additive
`bind_x509_ocsp_routed_issuer_to_routed_reviewed_server_path` boundary consumes
the fully routed
`AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation`. That proof retains
data-driven repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512
selection for the response signature, responder-certificate issuer signature,
and target-certificate issuer signature. The routed reviewed-server path
independently retains its two selected adjacency verifiers. The distinct
non-copyable result preserves both complete inputs and fresh exact-observation
validity for the same exact target and intermediate objects. It does not enter
routed responder-path policy.

The additive `reify_x509_ocsp_routed_issuer_reviewed_responder_path`
projection consumes that fully routed bound result and nominally reifies its
already-proven exact delegated responder-to-reviewed-root path. It retains the
complete non-copyable proof, including all five selected RSA signature routes
and responder/intermediate/target validity at the OCSP observation time, without
accepting caller-supplied certificate, path, algorithm, key, signature, time,
or policy input. It performs no new validation or external effect and stops
before routed responder DNS, no-check, or revocation policy.

`validate_x509_ocsp_routed_issuer_reviewed_responder_anchor_dns_policy` then
consumes that exact routed responder path and derives both policy inputs
internally: the fully routed authenticated responder certificate and the
selected reviewed root's exact optional out-of-band name-constraint bytes.
Entries without constraints succeed without reading the responder SAN.
Constrained entries decode the supported permitted DNS subset and exact
responder Subject Alternative Name under separate caller-owned limits, require
every responder `dNSName` to comply, and retain only count evidence. Failures
reuse the stable payload-free anchor-policy categories. No certificate, SAN,
root, path, algorithm, key, signature, or time can be supplied independently.

`validate_x509_ocsp_routed_issuer_reviewed_responder_intermediate_dns_policy`
consumes that complete routed anchor-policy proof and derives the exact routed
authenticated responder and exact routed authenticated intermediate internally.
An absent intermediate NameConstraints extension succeeds without reading the
responder SAN. A present extension must be critical and decode as the supported
permitted-and-excluded-DNS subset under separate caller-owned limits; every
responder `dNSName` must comply. Success retains only count evidence alongside
the complete prior proof, and failures reuse stable payload-free categories.

`validate_x509_ocsp_routed_issuer_reviewed_responder_no_check_policy` then
consumes that complete routed intermediate-DNS proof and derives the exact
authenticated responder certificate internally. It applies the same RFC 6960
semantic gate as the fixed path: one structurally unique
`id-pkix-ocsp-nocheck` extension whose value is exact DER `NULL`. Success
retains the complete fully routed proof, while missing or malformed values
reuse stable payload-free categories. A critical instance can reach this gate
only when certificate-wide role policy already recognized the OID.

`bind_x509_ocsp_routed_issuer_reviewed_responder_revocation` is the alternative
for a responder that does not use the no-check branch. It consumes the same
complete routed intermediate-DNS proof plus a separate fully routed,
authenticated OCSP target-revocation proof. The second proof must target the
same exact decoded responder, name the same exact retained intermediate, and
use the original server-status OCSP observation time. Target, issuer, then time
are checked in stable order; byte-equal independent decodes and cross-time
proofs fail with payload-free categories. Success retains both complete
non-copyable inputs. The boundary does not claim revocation policy for the
second proof's own signer or for the intermediate.

`bind_x509_ocsp_routed_issuer_reviewed_intermediate_revocation` is an
independent branch for the routed path's issuing CA. It consumes the complete
fully routed reviewed server-revocation proof plus a separate fully routed,
authenticated OCSP target-revocation proof. The second proof must target the
same exact decoded intermediate, name the same exact selected reviewed root,
and use the original server-status OCSP observation time. Target, issuer, then
time are checked in stable order; byte-equal independent decodes and
cross-time proofs fail with payload-free categories. Success retains both
complete non-copyable inputs. This status evidence does not substitute for
the responder no-check or authenticated responder-revocation branch and does
not establish revocation policy for the second proof's own responder.

`bind_x509_ocsp_routed_issuer_reviewed_no_check_intermediate_revocation`
composes the routed no-check branch with exact intermediate status without
duplicating the server proof. It consumes the complete routed responder
no-check proof plus a separate fully routed, authenticated OCSP proof, derives
the retained intermediate, selected reviewed root, and original observation
time internally, and applies the same stable target, issuer, then time checks.
Success retains both complete non-copyable inputs. It resolves the original
routed responder through RFC 6960 no-check policy, but does not establish
revocation policy for the added proof's own responder or complete revocation
policy.

`reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path`
then consumes that complete composition and nominally reifies the added status
proof's delegated responder-to-reviewed-root path. The retained proof already
contains the exact responder's certificate-wide role and critical-extension
policy, algorithm/key policy, OCSP-signing purpose, observation-time validity,
routed repository-owned RSA response and certificate-signature verification,
and the exact selected reviewed-root issuer. The projection accepts no
substitute certificate, path, root, algorithm, key, signature, status, or time
and performs no new validation or external effect. It stops before applying
the reviewed root's supported DNS constraints or establishing no-check or
separately authenticated revocation policy for that responder.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_anchor_dns_policy`
then consumes that exact added-status responder path and derives both policy
inputs internally: the added proof's fully routed authenticated responder and
the original proof's exact selected reviewed root, which the composition
already proves is the responder's issuer. Entries without out-of-band
constraints succeed without reading responder Subject Alternative Name.
Constrained entries decode the supported permitted-DNS subset and the exact
responder SAN under separate caller-owned ASN.1 limits, require every
responder `dNSName` to comply, and retain only count evidence. Failures reuse
the existing stable payload-free anchor-policy categories. This does not apply
excluded or non-DNS root constraints or establish no-check or separately
authenticated revocation policy for the added responder, complete revocation,
generalized path validation, TLS, HTTP, OAuth, or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_no_check_policy`
then consumes that complete reviewed-root DNS proof and derives the added
proof's exact authenticated responder internally. It requires the structurally
unique RFC 6960 `id-pkix-ocsp-nocheck` extension and exact DER `NULL` contents.
A critical instance can reach this semantic gate only when certificate-wide
role policy already recognized the OID; RFC 6960 recommends, but does not
require, non-critical marking. Success retains the entire prior proof, and
missing or malformed values reuse the existing stable payload-free categories.
This resolves only the trust-for-certificate-lifetime branch for the added
responder; separately authenticated responder revocation, complete revocation,
generalized path validation, TLS, HTTP, OAuth, and network readiness remain
unresolved.

The fixed-path `reify_x509_ocsp_reviewed_responder_path` projection consumes
the fully fixed RSA SHA-256 result and reifies its delegated responder-to-
reviewed-root path as a distinct non-copyable type for later policy
composition. Earlier safe
constructors already prove the exact responder-policy, responder-link, shared
issuer, and reviewed-path identities; this infallible projection performs no
additional validation. It preserves responder and intermediate validity at the
OCSP observation time together with the complete signed response,
target-revocation, server-path, path-length, and reviewed-root evidence.

`validate_x509_ocsp_reviewed_responder_anchor_dns_policy` then consumes that
exact responder-path proof and derives both inputs internally: the authenticated
responder certificate and the selected reviewed root's exact optional
out-of-band name-constraint bytes. Entries without constraints succeed without
reading the responder SAN. Constrained entries decode the supported permitted
DNS subset and the responder's exact Subject Alternative Name under separate
caller-owned limits, require every responder dNSName to comply, and retain only
count evidence. Failures expose stable payload-free categories.

`validate_x509_ocsp_reviewed_responder_intermediate_dns_policy` consumes that
complete anchor-policy proof and derives the exact authenticated responder and
the exact retained intermediate internally. An absent intermediate
NameConstraints extension succeeds without reading the responder SAN. A
present extension must be critical and decode as the supported permitted-and-
excluded-DNS subset under caller-owned limits; every responder `dNSName` must
comply. Success retains only count evidence alongside the complete prior proof,
and failures expose stable payload-free categories.

`validate_x509_ocsp_reviewed_responder_no_check_policy` then consumes that
complete intermediate-DNS proof and derives the exact authenticated responder
certificate internally. It requires the structurally unique RFC 6960
`id-pkix-ocsp-nocheck` extension and validates that its value is exact DER
`NULL`. The retained path already proves that the exact intermediate signed and
authorized the responder and that the responder was valid at the exact OCSP
observation time. A critical instance can reach this semantic gate only when
the prerequisite certificate-wide policy explicitly recognized its OID;
RFC 6960 recommends, but does not require, non-critical marking. Missing or
malformed values fail with stable payload-free categories.

## Boundary

This crate performs evidence composition, exact-time validity checks, the
bounded selected-anchor and exact-intermediate responder DNS policies, and the
RFC 6960 responder no-check or exact authenticated responder-revocation
branches plus the independent exact intermediate-revocation branch described
above and its exact no-check composition described above only.
The complete responder DNS and no-check policy sequence applies to both the
fully fixed RSA SHA-256 proof and the fully routed proof. The routed proof
retains every selected signature route through these semantic gates. The new
alternative branch binds one complete authenticated revocation proof to that
exact routed responder, intermediate, and observation time, but stops before
revocation policy for the second proof's signer and for the intermediate.
The independent intermediate branch binds another complete authenticated proof
to the exact retained intermediate, selected reviewed root, and observation
time. The no-check composition derives those same inputs through the consumed
no-check proof and retains the added proof. Its follow-up projection reifies
that proof's exact delegated responder-to-reviewed-root path; subsequent
policies apply supported reviewed-root DNS constraints and the no-check branch
to that exact added responder.
It does not perform new cryptography, search for or generalize certificate
paths, implement complete
RFC 5280 policy, apply excluded or non-DNS anchor constraints or non-DNS
intermediate constraints to the responder certificate, compose intermediate-
status evidence with the authenticated responder-revocation branch, implement
complete revocation policy, validate the additional proof's own responder
through the separately authenticated revocation alternative, interpret
unsupported OCSP extensions, acquire or cache OCSP data, read a clock,
implement TLS or HTTP, authorize OAuth transport, open a network connection,
or access a credential. The reviewed Mozilla root snapshot excludes OS,
enterprise, user-installed, and local roots. `webpki-roots` remains reviewed
public data only, not a TLS or validation implementation. Concrete OAuth HTTPS
remains unavailable.

## Public integration-fixture provenance

The checked-in integration fixture is static public evidence captured once on
2026-10-25. It is not fetched by tests and is not a production dependency.

One credential-free OpenSSL 3.6.3 probe used provider `www.apple.com`, SNI
`www.apple.com`, forced TLS 1.2, status request, certificate display, and
handshake-message tracing:

```text
openssl s_client -tls1_2 -connect www.apple.com:443 \
  -servername www.apple.com -status -showcerts -msg
```

The leaf and Apple intermediate are the exact certificates from that one
transcript. The complete stapled `OCSPResponse` is the exact 1,459-byte payload
from that transcript's TLS 1.2 `CertificateStatus` message after removing only
the handshake/status framing. It contains one delegated responder certificate
and one `good` SHA-1 `CertID` status signed with RSA PKCS #1 v1.5 SHA-256. The
response was produced at `2026-10-25T02:29:26Z`, with freshness through
`2026-10-25T18:29:25Z`; tests use explicit observation time
`2026-10-25T12:00:00Z` and never read a clock.

The DigiCert Global Root G2 DER came from DigiCert's official certificate
endpoint:

```text
https://cacerts.digicert.com/DigiCertGlobalRootG2.crt
```

OpenSSL independently verified the leaf for `sslserver`, the embedded
delegated responder for `ocsphelper`, and the response signature and target
status against the same Apple intermediate and DigiCert root before capture.
The repository tests then exercise only repository-owned parsing, hashing,
signature verification, certificate policy, path construction, reviewed-root
selection, name policy, revocation policy, and exact-object binding. System
OpenSSL was fixture-acquisition and independent-check tooling only; it supplies
no production HTTPS, certificate, OCSP, or trust authority.

- Apple leaf DER SHA-256:
  `cd563f5716c254cc1a574cb9f101a923ca50b2822c94c7c85575fff4b809e760`
- Apple intermediate DER SHA-256:
  `340ca5ba402d140b65a2c976e7ae8128a1505c29d190e0e034f59ccae7a92bc2`
- embedded delegated responder DER SHA-256:
  `b3b8febf1451170c65a5211252b74b1a972d09cfabfd5d3484de72aa0dd7110d`
- DigiCert Global Root G2 DER SHA-256:
  `cb3ccbb76031e5e0138f8dd39a23f9de47ffc35e43c1144cea27d46a5ab1cb5f`
- complete stapled OCSP response DER SHA-256:
  `757b42d0666a438b039a6c0e7ab646db5c70913c4e52dd2aceb645763d16ae79`

The acquisition touched only public provider endpoints, used no credential,
performed no signing or edit, and produced no remote mutation. The exact
temporary capture directory was moved to Trash after fixture verification.

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-reviewed-server-revocation-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-reviewed-server-revocation-binding --no-deps
```
