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

The stronger
`bind_x509_ocsp_routed_issuer_to_accumulated_name_policy_reviewed_server_path`
boundary consumes the same fully routed authenticated OCSP proof together with
`ValidatedAccumulatedNamePolicyReviewedX509RsaConstructedPathSubordinateAnchorNameConstraintsPolicy`.
That path retains its exact validated Name-value policy, reviewed root and
certificate-wide root role, arbitrary-depth routed signatures, complete
intermediate and server policy, construction-derived path-length state, and
the supported target and subordinate Name-constraint evaluations. The binder
derives the target only from retained complete server policy and derives its
target-nearest issuer only from the retained depth-zero routed signature. It
then applies the same stable target-identity, issuer-identity, target-validity,
and issuer-validity checks at the OCSP observation time. Success retains both
complete non-copyable inputs and the two exact-time validity tokens.

The additive
`bind_x509_ocsp_routed_issuer_to_signed_accumulated_name_policy_reviewed_server_path`
boundary consumes that same fully routed authenticated OCSP proof together
with the complete
`BoundX509RsaReviewedServerPathAnchorSelfSignature`. It derives the target and
target-nearest issuer only through the signed wrapper's retained strongest
Name-policy path, applies the same stable exact-object and observation-time
validity checks, and retains the complete path plus reviewed-anchor
self-signature evidence intact in the existing generic non-copyable result.
The earlier path-only entry point remains available and unchanged. Tests use
the public Apple/DigiCert fixture to prove success and reject byte-equal
independently decoded target and issuer objects.

The fully routed strong binding also implements the sealed
`X509OcspReviewedServerRevocationTargetEvidence` projection. Only the
repository-owned binding can implement that interface, and it returns the
exact decoded target already retained by the authenticated OCSP proof. The
direct responder-no-check complete fixed-path wrapper carries that projection
through its complete retained proof and exposes the same exact object without
accepting a detached certificate, callback, path, status result, or trust
assertion. This prepares a later destination-to-complete-revocation binding;
it does not itself bind an HTTPS destination.

The nested responder-no-check complete fixed-path wrapper now carries the same
sealed projection through its deeper retained proof graph. It exposes only the
exact server target retained by the original reviewed-server proof, and a
byte-equal independently decoded certificate remains a different object. This
projection accepts no detached certificate, callback-shaped claim, path,
status result, or trust assertion. The separate
`https-reviewed-server-path-binding` package now binds this nested completion
to its exact HTTPS destination without widening that input surface.

The double-nested responder-no-check complete fixed-path wrapper carries the
same sealed projection through both retained responder-revocation layers. It
likewise exposes only the original reviewed-server proof's exact target and
a byte-equal independently decoded certificate remains a different object.
The projection accepts no detached certificate, callback-shaped claim, path,
status result, or trust assertion. Binding an HTTPS destination to this
double-nested completion remains separate work.

This ordered signed-path bridge performs no new parsing or signature
verification and no credential access or editing, signing, storage, clock
acquisition, provider action, audit or trace emission, I/O, network access, or
other external effect. It does not fetch status, add trust-anchor semantics,
authorize the delegated responder path, establish status for every non-root
certificate, complete Name, path, or revocation validation, provide TLS or
HTTP, authorize OAuth transport, or grant network authority. Concrete HTTPS
and live-provider readiness remain unavailable.

The sealed target projections likewise perform no parsing, validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit or trace emission, I/O, network access, or
other external effect. The remaining complete fixed-path revocation branches
and the double-nested HTTPS composition remain separate work.

This stronger bridge performs no credential access or editing, signing,
storage, clock acquisition, provider action, audit or trace emission, I/O,
network access, or other external effect. It does not fetch status, authorize
or reify the delegated responder path, apply responder DNS or no-check policy,
establish status for every non-root certificate, complete Name or path
validation, validate the anchor self-signature, provide TLS or HTTP, authorize
OAuth transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

`reify_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_path`
nominally projects that exact strong bridge into the authenticated responder's
already-proven responder-to-issuer-to-reviewed-root suffix. It retains the
complete non-copyable bridge and accepts no detached certificate, path,
algorithm, key, signature, time, or policy input. The projection performs no
new validation or external effect and stops before responder DNS, no-check,
responder or intermediate revocation, complete revocation or Name/path
validation, TLS, HTTP, OAuth, network, or live-provider readiness.

`validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_anchor_dns_policy`
consumes that exact strong responder path and derives both policy inputs
internally: the fully routed authenticated responder certificate and the
selected Mozilla-reviewed root's exact optional out-of-band name-constraint
bytes retained by the strongest Name-policy path. Entries without constraints
succeed without reading the responder SAN. Constrained entries decode the
supported permitted-DNS subset and exact responder Subject Alternative Name
under separate caller-owned limits, require every responder `dNSName` to
comply, and retain only count evidence. The entry point reuses the existing
private evaluator and stable payload-free anchor-policy errors. No certificate,
SAN, root, path, algorithm, key, signature, time, or policy can be supplied
independently. This policy performs no external effect and stops before
intermediate DNS, no-check, responder or intermediate revocation, complete
revocation or Name/path validation, TLS, HTTP, OAuth, network, or live-provider
readiness.

`validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_intermediate_dns_policy`
consumes that complete strong anchor-policy proof and derives both the exact
fully routed authenticated responder and the exact target-nearest intermediate
through the retained non-copyable evidence. An absent intermediate
NameConstraints extension succeeds without reading responder Subject
Alternative Name. A present extension must be critical and decode as the
supported permitted-and-excluded-DNS subset under separate caller-owned
limits; every responder `dNSName` must comply. The entry point reuses the
existing private evaluator and stable payload-free intermediate-policy errors,
and retains only count evidence with the complete prior proof. No responder,
SAN, intermediate, path, algorithm, key, signature, status, or time can be
supplied independently. This policy performs no external effect and stops
before no-check, responder or intermediate revocation, complete revocation or
Name/path validation, TLS, HTTP, OAuth, network, or live-provider readiness.

`validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_no_check_policy`
consumes that complete strong intermediate-DNS proof and derives the exact
fully routed authenticated responder only through its retained non-copyable
evidence. The responder must carry one structurally unique
`id-pkix-ocsp-nocheck` extension whose value is exact DER `NULL`; the entry
point reuses the existing private evaluator and stable payload-free errors. A
critical instance can reach this gate only when certificate-wide role policy
already recognized the OID. No responder, extension, path, algorithm, key,
signature, status, or time can be supplied independently. This policy performs
no external effect and stops before responder or intermediate revocation,
complete revocation or Name/path validation, TLS, HTTP, OAuth, network, or
live-provider readiness.

`bind_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_revocation`
is the separate authenticated-revocation alternative to that strongest-path
no-check gate. It consumes the same complete intermediate-DNS proof plus one
fully routed authenticated OCSP target-revocation proof, then derives the
expected responder, target-nearest intermediate, and original server-status
observation time exclusively through the retained evidence. Exact decoded-
object identity and field-wise time equality are required in stable target,
issuer, then time order. Success retains both complete non-copyable inputs;
no raw certificate, status, freshness endpoint, path, key, signature, or time
can be substituted. This composition performs no external effect and stops
before policy for the added proof's responder or the retained intermediate,
complete revocation or Name/path validation, TLS, HTTP, OAuth, network, or
live-provider readiness.

That strongest responder-revocation result is now also admitted directly to
the existing generic retained-intermediate revocation binding boundary. A
private repository-owned evidence adapter derives the target-nearest
intermediate, selected reviewed root, and original server-status observation
time only through the retained strong proof; callers cannot substitute a
certificate, root, status proof, or time. This enables the existing stable
target-issuer-time binding for separately authenticated intermediate status
without adding a public API or external effect. Policy for either added OCSP
proof's own responder, complete revocation or Name/path validation, TLS, HTTP,
OAuth, network, and live-provider readiness remain unavailable.

That strongest-path result is admitted directly to the existing generic
`reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path`
projection and
`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_anchor_dns_policy`
gate. The private evidence adapter derives the selected reviewed root's exact
optional constraint bytes only through the retained arbitrary-depth Name-policy
proof; it accepts no detached certificate, root, constraint, path, signature,
status, or time. This closes only the selected-root DNS composition boundary
for the added responder and stops before intermediate DNS, no-check, further
revocation, complete Name/path validation, TLS, HTTP, OAuth, network, or
live-provider readiness.

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

`reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_path`
then consumes that complete separately authenticated branch and nominally
reifies the added revocation proof's own delegated responder-to-intermediate-
to-reviewed-root path. The infallible non-copyable wrapper retains both the
original routed responder policy and the exact authenticated responder-status
proof; it accepts no substitute certificate, path, root, algorithm, key,
signature, status, or time. This projection performs no new validation or
external effect and stops before reviewed-root or intermediate name policy,
no-check or separately authenticated revocation policy for that added
responder, complete revocation, generalized path validation, TLS, HTTP, OAuth,
or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_anchor_dns_policy`
then consumes that exact added responder path and derives both inputs through
private repository-owned evidence: the separately authenticated proof's fully
routed responder and the original proof's exact selected reviewed-root
constraints. Entries without out-of-band constraints succeed without reading
responder Subject Alternative Name. Constrained entries apply the supported
permitted-DNS subset under separate caller-owned parse limits and retain only
count evidence. Stable failures expose no certificate, name, constraint,
response, key, signature, status, root, or time payload. This stops before
intermediate name constraints, no-check or separately authenticated revocation
policy for the added responder, complete revocation, generalized path
validation, TLS, HTTP, OAuth, or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_intermediate_dns_policy`
then consumes that complete reviewed-root DNS proof and derives the separately
authenticated proof's exact responder and exact retained intermediate through
private repository-owned evidence. An intermediate without NameConstraints
succeeds without reading responder Subject Alternative Name. A present
NameConstraints extension must be critical; its supported permitted and
excluded DNS subtrees are applied under separate caller-owned parse limits,
and success retains only count evidence. Stable failures expose no certificate,
name, constraint, response, key, signature, status, root, or time payload. This
stops before no-check or separately authenticated revocation policy for the
added responder, complete revocation, generalized path validation, TLS, HTTP,
OAuth, or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_policy`
then consumes that complete root-and-intermediate DNS proof and derives the
separately authenticated proof's exact responder through private repository-
owned evidence. It requires one structurally unique `id-pkix-ocsp-nocheck`
extension with exact DER `NULL` contents and retains the complete non-copyable
proof on success. Stable failures expose no certificate, extension, response,
key, signature, status, root, or time payload. This resolves only RFC 6960's
trust-for-certificate-lifetime branch and stops before separately authenticated
revocation for this responder, complete revocation, generalized path
validation, TLS, HTTP, OAuth, or network readiness.

`bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_revocation`
then consumes that complete no-check proof plus a separately authenticated,
fully routed proof for the exact retained intermediate. It derives the exact
intermediate, selected reviewed root, and original observation time only
through the consumed no-check proof, reuses the stable target-then-issuer-then-
time binding policy, and retains both complete non-copyable inputs on success.
This resolves the original responder through separately authenticated status,
resolves that status proof's own responder through RFC 6960 no-check policy,
and authenticates status for the exact retained intermediate. It stops before
revocation policy for the intermediate-status proof's own responder, complete
revocation, generalized path validation, TLS, HTTP, OAuth, or network
readiness.

`reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_path`
then projects the retained intermediate-status proof's exact authenticated
responder-to-reviewed-root path without accepting any substitute evidence or
performing more parsing or validation. The generic path wrapper retains the
complete responder-of-responder no-check plus intermediate-status composition
under a source-specific accessor. It stops before reviewed-root DNS policy for
that responder, no-check or separately authenticated revocation policy for it,
complete revocation, generalized path validation, TLS, HTTP, OAuth, or network
readiness.

`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_responder_no_check_intermediate_status_responder_anchor_dns_policy`
then consumes that exact nested-source path and derives both the intermediate-
status proof's fully routed authenticated responder and the original proof's
selected reviewed-root constraints through private repository-owned evidence.
Entries without constraints succeed without reading responder Subject
Alternative Name. Constrained entries decode the supported permitted-DNS
subset and exact responder SAN under separate caller-owned ASN.1 limits,
require every responder `dNSName` to comply, and retain only count evidence.
The complete nested no-check plus intermediate-status composition remains
available to the existing added-responder no-check and separately authenticated
revocation policies. Stable failures expose no certificate, name, constraint,
response, key, signature, status, root, or time payload. This does not apply
excluded or non-DNS root constraints, establish complete revocation policy,
generalize path validation, or provide TLS, HTTP, OAuth, or network readiness.

`complete_x509_ocsp_routed_issuer_reviewed_nested_no_check_fixed_path_revocation_policy`
then consumes the exact branch after the intermediate-status responder also
passes RFC 6960 no-check policy. Its input type proves that every non-root
certificate in this one retained fixed chain is covered by authenticated good-
and-fresh OCSP status or exact no-check policy; the selected Mozilla-reviewed
root remains the trust anchor. The infallible nominal wrapper retains the full
composition and accepts no substitute evidence. It does not generalize path
construction or validation, acquire or refresh revocation data, read a clock,
or provide TLS, HTTP, OAuth, or network readiness.

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
composes either admitted routed no-check branch with exact intermediate status
without duplicating the server proof. Private repository-owned evidence admits
both the fixed-depth and strongest Name-policy-backed server proofs while
preventing external substitute implementations. The entry point consumes the
complete responder no-check proof plus a separate fully routed, authenticated
OCSP proof, derives the retained intermediate, selected reviewed root, and
original observation time internally, and applies the same stable target,
issuer, then time checks. Success retains both complete non-copyable inputs.
It resolves the original routed responder through RFC 6960 no-check policy,
but does not establish revocation policy for the added proof's own responder
or complete revocation policy, TLS, HTTP, OAuth, network, or live-provider
readiness.

`bind_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_revocation`
provides the corresponding composition for an original responder that uses
separately authenticated revocation instead of no-check. Repository-owned
private evidence interfaces derive the exact retained intermediate, selected
reviewed root, and original observation time from the complete responder-
revocation branch, then require a separate fully routed authenticated proof to
name those exact objects and time. Target, issuer, then time are checked in
stable order. Success retains both complete non-copyable branches, while
failures reuse the existing payload-free intermediate-binding categories.
This does not establish revocation policy for either added proof's own
responder or provide complete revocation, generalized path validation, TLS,
HTTP, OAuth, or network readiness.

`reify_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_path`
then projects that authenticated-original-responder composition's exact added
status responder-to-reviewed-root path. The infallible wrapper retains both
complete authenticated revocation branches and accepts no substitute
certificate, path, root, algorithm, key, signature, status, or time. The
defaulted source parameter preserves the existing no-check projection's public
type shape.

`validate_x509_ocsp_routed_issuer_reviewed_responder_revocation_intermediate_status_responder_anchor_dns_policy`
derives the added proof's authenticated responder and the original proof's
selected reviewed-root constraints through private repository-owned evidence.
It applies the same bounded permitted-DNS policy as the no-check-source path.
The resulting proof is consumed unchanged by the existing added-responder
no-check and separately authenticated revocation functions. This resolves
policy routing for the added responder only; it does not establish policy for
either additional proof's own responder or complete revocation.

`reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_path`
then consumes the complete no-check composition and nominally reifies the
added status proof's delegated responder-to-reviewed-root path. The retained proof already
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

`complete_x509_ocsp_routed_issuer_reviewed_direct_no_check_fixed_path_revocation_policy`
then nominally completes revocation policy for the exact direct branch. Its
input type proves that authenticated good-and-fresh OCSP status covers the
server target and retained intermediate, while exact RFC 6960 no-check policy
covers the original responder and added intermediate-status responder. This
also admits the strongest Name-policy-backed original proof through the same
retained generic type path. The selected Mozilla-reviewed root remains the
trust anchor. The infallible non-copyable wrapper retains the complete proof
and accepts no substitute evidence. It performs no parsing, cryptography,
clock acquisition, credential or provider operation, audit or trace emission,
I/O, or network effect. It does not generalize revocation or path construction
and validation, acquire or refresh revocation data, or provide TLS, HTTP,
OAuth, network, or live-provider readiness.

`bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation`
is the alternative for an added responder that does not use no-check. It
consumes the same complete reviewed-root DNS proof plus a separate fully
routed authenticated OCSP target-revocation proof. Both proof interfaces are
repository-owned private bounds: downstream callers can use the concrete
authenticated proof types but cannot implement substitute evidence. The
second proof must target the added proof's same exact responder, name its same
exact selected reviewed-root issuer, and use its same explicit observation
time. Target, issuer, then time are checked in stable order. Success retains
both complete non-copyable inputs, while failures reuse the existing stable
payload-free categories. This does not establish revocation policy for the
second proof's own responder or complete revocation, generalized path
validation, TLS, HTTP, OAuth, or network readiness.

That alternative branch can then use
`reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_path`
to project the second proof's own exact delegated responder-to-reviewed-root
path. The infallible non-copyable wrapper preserves the complete generic source
and authenticated revocation proof without accepting substitute evidence or
performing more validation. It stops before reviewed-root or intermediate DNS
policy for the delegated responder, no-check or separately authenticated
revocation policy for it, complete revocation, generalized path validation,
TLS, HTTP, OAuth, or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_anchor_dns_policy`
then consumes that exact path and derives both policy inputs internally: the
added revocation proof's authenticated delegated responder and the selected
Mozilla-reviewed root's optional out-of-band constraint bytes. Entries without
constraints succeed without reading responder Subject Alternative Name.
Constrained entries decode the supported permitted-DNS subset and the exact
responder SAN under separate caller-owned ASN.1 limits, require every responder
`dNSName` to comply, and retain only count evidence. Stable failures expose no
certificate, name, constraint, response, key, signature, status, root, or time
payload. This does not apply excluded or non-DNS root constraints or the
retained intermediate's constraints, establish no-check or separately
authenticated revocation policy for the delegated responder, complete
revocation, generalized path validation, TLS, HTTP, OAuth, or network
readiness.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_policy`
then consumes that complete reviewed-root DNS proof and derives the added
revocation proof's exact authenticated delegated responder only through the
retained path. It requires one structurally unique `id-pkix-ocsp-nocheck`
extension whose value is exact DER `NULL`; RFC 6960 recommends but does not
require that extension to be non-critical, so certificate-wide role policy
must already have recognized a critical instance. Success retains the entire
generic source and non-copyable proof. This resolves only the no-check branch
for that exact responder and stops before separately authenticated recursive
responder revocation, complete revocation, generalized path validation, TLS,
HTTP, OAuth, or network readiness.

`bind_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation`
provides the separately authenticated alternative for that exact delegated
responder. It consumes the same complete reviewed-root DNS proof plus another
fully routed authenticated OCSP target-revocation proof, derives the added
proof's authenticated responder, exact selected reviewed-root issuer, and
original observation time only through the retained evidence, and requires
the recursive proof to name those same exact objects at that exact time.
Target, issuer, then time are checked in stable order with the existing
payload-free failure categories. Success retains the entire generic source
and both non-copyable authenticated proof layers. It stops before revocation
policy for the recursive proof's own responder, complete revocation,
generalized path validation, TLS, HTTP, OAuth, or network readiness.

`reify_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_path`
then projects the recursive proof's own exact delegated responder-to-reviewed-
root path. The infallible non-copyable wrapper retains the complete generic
source and both authenticated proof layers without accepting a substitute
certificate, path, root, algorithm, key, signature, status, or time. It
performs no new validation or external effect and stops before reviewed-root
name policy, no-check or separately authenticated revocation policy for that
responder, complete revocation, generalized path validation, TLS, HTTP, OAuth,
or network readiness.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_anchor_dns_policy`
then consumes that exact recursive responder path and derives both policy
inputs internally: the recursive proof's authenticated delegated responder
and the selected Mozilla-reviewed root's optional out-of-band constraint
bytes. Entries without constraints succeed without reading responder Subject
Alternative Name. Constrained entries decode the supported permitted-DNS
subset and exact responder SAN under separate caller-owned ASN.1 limits,
require every responder `dNSName` to comply, and retain only count evidence.
Stable failures expose no certificate, name, constraint, response, key,
signature, status, root, or time payload. This does not apply excluded or
non-DNS root constraints or intermediate constraints, establish no-check or
further separately authenticated revocation policy for that responder,
complete revocation, generalized path validation, TLS, HTTP, OAuth, or network
readiness.

`validate_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_revocation_responder_no_check_policy`
then consumes that complete reviewed-root DNS proof and derives the recursive
proof's exact authenticated responder only through the retained non-copyable
evidence. It requires one structurally unique `id-pkix-ocsp-nocheck` extension
whose value is exact DER `NULL`. A critical instance can reach this semantic
gate only when certificate-wide role policy already recognized its OID; RFC
6960 recommends, but does not require, non-critical marking. Success retains
the complete proof and resolves only the no-check branch for that exact
recursive responder. Separately authenticated further recursion, complete
revocation, generalized path validation, TLS, HTTP, OAuth, and network
readiness remain unavailable.

`complete_x509_ocsp_routed_issuer_reviewed_recursive_responder_no_check_fixed_path_revocation_policy`
then nominally completes revocation policy for this exact default-source
recursive branch. Its input type proves that the server target, original
responder, intermediate, added intermediate-status responder, that responder's
revocation proof signer, and the recursive revocation proof's own signer are
each covered by authenticated good-and-fresh OCSP status or exact RFC 6960
no-check policy. The selected Mozilla-reviewed root remains the trust anchor.
The infallible non-copyable wrapper retains the complete proof and accepts no
substitute evidence. It performs no parsing, cryptography, clock acquisition,
credential or provider operation, audit or trace emission, I/O, or network
effect. It does not generalize path construction or validation, acquire or
refresh revocation data, or provide TLS, HTTP, OAuth, or network readiness.

`complete_x509_ocsp_routed_issuer_reviewed_intermediate_status_responder_revocation_responder_no_check_fixed_path_revocation_policy`
then nominally completes revocation policy for this exact default-source
alternative branch. Its input type proves that the server target, original
responder, intermediate, added intermediate-status responder, and the added
responder-revocation proof's own responder are each covered by authenticated
good-and-fresh OCSP status or exact RFC 6960 no-check policy. The selected
Mozilla-reviewed root remains the trust anchor. The infallible non-copyable
wrapper retains the complete proof and accepts no substitute evidence. It does
not generalize path construction or validation, acquire or refresh revocation
data, read a clock, or provide TLS, HTTP, OAuth, or network readiness.

`complete_x509_ocsp_routed_issuer_reviewed_double_nested_no_check_fixed_path_revocation_policy`
provides the corresponding source-specific completion when the original
responder uses separately authenticated revocation and that proof's own
responder has already passed exact RFC 6960 no-check policy. The retained
alternative branch additionally authenticates revocation for the added
intermediate-status responder and applies exact no-check policy to that proof's
own responder. Its input type therefore covers every non-root certificate in
this one double-nested fixed path while excluding the unresolved generic
authenticated-recursion source. The selected Mozilla-reviewed root remains the
trust anchor. This nominal composition performs no new parsing, cryptography,
clock acquisition, I/O, or external effect and does not generalize path
construction or validation, acquire or refresh revocation data, or provide
TLS, HTTP, OAuth, or network readiness.

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
above and its exact no-check and authenticated-responder compositions only.
The complete responder DNS and no-check policy sequence applies to both the
fully fixed RSA SHA-256 proof and the fully routed proof. The routed proof
retains every selected signature route through these semantic gates. The new
alternative branch binds one complete authenticated revocation proof to that
exact routed responder, intermediate, and observation time, but stops before
revocation policy for the second proof's signer and for the intermediate.
The independent intermediate branch binds another complete authenticated proof
to the exact retained intermediate, selected reviewed root, and observation
time. Both compositions derive those same inputs through the consumed
responder-policy proof and retain the added proof. Both follow-up projections
reify the added proof's exact delegated responder-to-reviewed-root path;
subsequent policies apply supported reviewed-root DNS constraints and either
the no-check or separately authenticated revocation branch to that exact added
responder.
The direct no-check complete fixed-path result can project the exact server
target only through its retained sealed repository-owned proof. It accepts no
independent certificate, callback, path, status result, or trust assertion and
does not bind an HTTPS destination.
It does not perform new cryptography, search for or generalize certificate
paths, implement complete
RFC 5280 policy, apply excluded or non-DNS anchor constraints or non-DNS
intermediate constraints to the responder certificate, implement complete
revocation policy outside the explicitly completed fixed branches, validate
either additional revocation proof's own responder,
interpret unsupported OCSP extensions,
acquire or cache OCSP data, read a clock, implement TLS or HTTP, authorize
OAuth transport, open a network connection, or access a credential. The
reviewed Mozilla root snapshot excludes OS, enterprise, user-installed, and
local roots. `webpki-roots` remains reviewed public data only, not a TLS or
validation implementation. Concrete OAuth HTTPS remains unavailable.

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
