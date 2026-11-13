# Changelog

## Unreleased

- Apply exact RFC 6960 no-check policy to the delegated responder retained by
  the strongest Name-policy path/fully routed OCSP bridge after both responder
  DNS-policy gates.
- Derive the responder only through the consumed non-copyable proof, reuse the
  existing redacted exact-DER evaluator, and stop before further revocation,
  complete path validation, TLS, HTTP, OAuth, or network readiness.
- Apply the exact target-nearest intermediate's supported critical permitted-
  and-excluded DNS constraints to the exact delegated responder retained by
  the strongest Name-policy path/fully routed OCSP bridge.
- Derive both certificates only through the consumed non-copyable strong
  anchor-policy proof, reuse the existing redacted evaluator, and stop before
  no-check, revocation completion, TLS, HTTP, OAuth, or network readiness.
- Apply the selected Mozilla-reviewed root's supported out-of-band permitted-
  DNS constraints to the exact delegated responder retained by the strongest
  Name-policy path/fully routed OCSP bridge.
- Derive both policy inputs only through the non-copyable strong projection,
  reuse the existing redacted evaluator, and stop before intermediate DNS,
  no-check, revocation completion, TLS, HTTP, OAuth, or network readiness.
- Reify the authenticated responder-to-reviewed-root suffix retained by the
  strongest Name-policy path/fully routed OCSP bridge without detached inputs.
- Bind fully routed authenticated OCSP target status to the strongest supported
  Name-policy-backed arbitrary-depth reviewed-server path by exact target and
  target-nearest issuer object identity.
- Retain both complete non-copyable inputs and exact-observation target/issuer
  validity while stopping before responder authorization, complete revocation,
  complete Name/path validation, TLS, HTTP, OAuth, or network readiness.
- Complete revocation policy for the exact default-source recursive fixed path
  after every non-root certificate is covered by authenticated good-and-fresh
  status or exact RFC 6960 no-check policy.
- Retain the complete non-copyable proof while stopping before generalized
  path validation, revocation acquisition or refresh, TLS, HTTP, or OAuth.
- Apply exact RFC 6960 no-check policy to the recursive OCSP proof's own
  authenticated delegated responder after its selected reviewed-root DNS
  policy.
- Derive the responder only through the retained non-copyable proof and stop
  before further authenticated recursion, complete revocation, generalized
  path validation, TLS, HTTP, or OAuth readiness.
- Apply the selected reviewed root's supported out-of-band permitted-DNS
  constraints to the recursive OCSP revocation proof's exact delegated
  responder while retaining both authenticated proof layers.
- Derive the responder and root constraints only through the consumed generic
  source, retain count-only evidence, and stop before intermediate name,
  no-check, further revocation, complete revocation, TLS, HTTP, or OAuth policy.
- Reify the recursive OCSP revocation proof's own exact delegated responder-
  to-reviewed-root path while retaining the complete generic source and both
  authenticated proof layers.
- Accept no substitute certificate, path, root, algorithm, key, signature,
  status, or time, and stop before name, no-check, recursive revocation,
  complete revocation, generalized path validation, TLS, HTTP, or OAuth policy.
- Bind a separately authenticated routed OCSP proof to the alternative added
  revocation proof's own exact delegated responder, selected reviewed-root
  issuer, and original observation time in stable target-issuer-time order.
- Retain the complete generic non-copyable source and stop before policy for
  the recursive proof's own responder, complete revocation, generalized path
  validation, TLS, HTTP, or OAuth readiness.
- Complete revocation policy for the double-nested alternative fixed path
  whose original and added responder-revocation proofs both carry exact RFC
  6960 no-check policy for their own responders.
- Restrict the nominal completion to the fully covered source-specific proof
  and stop before generic authenticated recursion, generalized path validation,
  revocation acquisition or refresh, TLS, HTTP, or OAuth readiness.
- Complete revocation policy for the alternative added-responder fixed path
  after every non-root certificate is covered by authenticated status or exact
  RFC 6960 no-check policy.
- Restrict the infallible non-copyable completion to the fully covered default
  source and stop before generalized path validation, revocation acquisition
  or refresh, TLS, HTTP, or OAuth readiness.
- Apply RFC 6960 no-check policy to the alternative added revocation proof's
  own exact delegated responder after its selected reviewed-root DNS policy.
- Retain the complete generic non-copyable proof while stopping before
  separately authenticated recursive responder revocation, complete
  revocation, generalized path validation, TLS, HTTP, or OAuth readiness.
- Apply the selected reviewed root's supported out-of-band permitted-DNS
  constraints to the alternative added revocation proof's exact delegated
  responder, retaining only count evidence and the complete generic source.
- Stop before intermediate constraints, responder no-check or separately
  authenticated revocation, complete revocation, generalized path validation,
  TLS, HTTP, or OAuth readiness.
- Reify the alternative intermediate-status responder-revocation proof's own
  exact delegated responder-to-reviewed-root path while retaining the complete
  generic source and authenticated proof.
- Stop before name policy or revocation policy for that delegated responder,
  complete revocation, generalized path validation, TLS, HTTP, or OAuth.
- Complete revocation policy for the exact retained nested no-check fixed path
  after every non-root certificate is covered by authenticated good-and-fresh
  status or RFC 6960 no-check policy.
- Retain the complete non-copyable proof while stopping before generalized
  path validation, revocation acquisition or refresh, TLS, HTTP, or OAuth.
- Apply the selected reviewed root's supported out-of-band permitted-DNS
  constraints to the nested no-check composition's exact intermediate-status
  responder, retaining only count evidence.
- Preserve the complete non-copyable source for downstream no-check or
  separately authenticated revocation policy and stop before complete
  revocation, generalized path validation, TLS, HTTP, or OAuth readiness.
- Reify the exact intermediate-status responder-to-reviewed-root path while
  retaining the complete responder-of-responder no-check plus intermediate-
  status composition.
- Stop before reviewed-root DNS or revocation policy for that responder,
  complete revocation, generalized path validation, TLS, HTTP, or OAuth.
- Compose the responder-of-responder RFC 6960 no-check proof with separately
  authenticated status for the exact retained intermediate at the original
  observation time.
- Retain both complete non-copyable inputs and stop before policy for the
  intermediate-status proof's own responder or complete revocation readiness.
- Apply the repository-owned RFC 6960 no-check evaluator to the separately
  authenticated responder-revocation proof's own exact responder after its
  root and intermediate DNS policies.
- Retain the complete non-copyable proof and stop before recursive responder
  revocation, complete revocation, TLS, HTTP, or OAuth readiness.
- Apply the separately authenticated responder-revocation proof's exact
  intermediate critical permitted and excluded DNS constraints to that proof's
  exact responder, retaining only count evidence.
- Reuse the repository-owned bounded evaluator and stop before no-check,
  recursive responder revocation, complete revocation, TLS, HTTP, or OAuth
  readiness.
- Apply the selected reviewed root's supported out-of-band permitted-DNS
  constraints to the separately authenticated original responder-revocation
  proof's own exact responder.
- Derive both policy inputs through the consumed non-copyable path, retain only
  count evidence, and stop before intermediate name constraints, no-check,
  recursive revocation, complete revocation, TLS, HTTP, or OAuth readiness.
- Reify the separately authenticated original responder-revocation proof's own
  exact delegated responder-to-intermediate-to-reviewed-root path while
  retaining the complete non-copyable branch.
- Accept no substitute certificate, path, root, algorithm, key, signature,
  status, or time, and stop before name, no-check, recursive revocation,
  complete revocation, generalized path validation, TLS, HTTP, or OAuth policy.
- Reify the added intermediate-status responder path from the original
  separately authenticated responder-revocation branch while retaining both
  complete authenticated revocation proofs under repository-owned evidence.
- Feed that projected path through the existing reviewed-root DNS, RFC 6960
  no-check, and separately authenticated added-responder revocation policies
  without accepting substitute certificates, roots, status, or time.
- Preserve defaulted public type parameters for the existing no-check source
  and stop before either added proof's own responder policy, complete
  revocation, generalized path validation, TLS, HTTP, or OAuth readiness.
- Compose the separately authenticated original responder-revocation branch
  with exact intermediate status by deriving the retained intermediate,
  selected reviewed root, and original observation time through private
  repository-owned evidence interfaces.
- Retain both complete non-copyable branches with stable payload-free target,
  issuer, and time failures, while leaving both added responders' revocation
  policy, downstream projection, complete revocation, TLS, HTTP, and OAuth
  readiness unresolved.
- Bind a separate fully routed authenticated OCSP proof to the added
  intermediate-status proof's exact responder, selected reviewed root, and
  original observation time after reviewed-root DNS policy.
- Retain both complete non-copyable proofs with stable payload-free target,
  issuer, and time failures, while leaving the second proof's responder policy,
  complete revocation, TLS, HTTP, and OAuth readiness unresolved.
- Apply the RFC 6960 no-check semantic gate to the added intermediate-status
  proof's exact authenticated responder after reviewed-root DNS policy.
- Derive the responder only through the retained proof, reuse exact DER `NULL`
  validation and payload-free failures, and stop before the authenticated
  revocation alternative, complete revocation, TLS, HTTP, or OAuth readiness.
- Apply the selected reviewed root's supported out-of-band DNS constraints to
  the added intermediate-status proof's exact fully routed responder under
  separate caller-owned parse limits and with payload-free failures.
- Derive the responder and root constraints only through the retained proof,
  preserve count-only evidence, and stop before added-responder no-check or
  authenticated revocation, complete revocation, TLS, HTTP, or OAuth readiness.
- Reify the added intermediate-status proof's exact delegated responder-to-
  reviewed-root path from the complete routed no-check composition.
- Retain all prior role, algorithm, routed RSA, issuer-identity, and exact-time
  evidence without new validation or effects, while leaving reviewed-root DNS
  and that responder's no-check or authenticated-revocation policy unresolved.
- Compose the fully routed responder no-check proof with separately
  authenticated intermediate status by deriving the retained intermediate,
  selected reviewed root, and original observation time internally.
- Retain both complete non-copyable inputs with the existing stable payload-
  free target, issuer, and time failures, while leaving the added proof's own
  responder revocation and complete revocation policy unresolved.
- Bind a separate fully routed authenticated OCSP proof to the exact retained
  intermediate, selected reviewed root, and original server-status observation
  time, with stable payload-free target, issuer, and time mismatch failures.
- Retain both complete non-copyable proofs without new cryptography or external
  effects, while leaving responder-policy composition, the second proof's own
  responder revocation, generalized path validation, TLS, HTTP, and OAuth
  unresolved.
- Bind a separate fully routed authenticated OCSP proof to the exact routed
  responder, retained intermediate, and original server-status observation
  time, with stable payload-free target, issuer, and time mismatch failures.
- Retain both complete non-copyable proofs without new cryptography or external
  effects, while leaving the second proof's signer revocation, intermediate
  revocation, generalized path validation, TLS, HTTP, and OAuth unresolved.
- Apply the RFC 6960 no-check semantic gate to the exact authenticated
  responder retained by the fully routed intermediate-DNS proof.
- Reuse exact DER `NULL` validation and stable payload-free failures, preserve
  every routed signature choice, and stop before responder or intermediate
  revocation, TLS, HTTP, or OAuth readiness.
- Apply the exact routed intermediate's supported critical permitted-and-
  excluded-DNS constraints to the exact routed authenticated responder, under
  separate caller-owned parse limits and with payload-free failures.
- Derive both certificates only through the consumed routed anchor-policy proof,
  retain count-only evidence, and stop before routed no-check, revocation, TLS,
  HTTP, or OAuth readiness.
- Apply the selected reviewed root's supported out-of-band DNS constraints to
  the exact authenticated responder retained by the fully routed proof, under
  separate caller-owned parse limits and with payload-free failures.
- Derive both certificate inputs exclusively from the consumed routed proof,
  retain count-only DNS evidence, and stop before routed intermediate DNS,
  no-check, revocation, TLS, HTTP, or OAuth readiness.
- Add an infallible nominal projection from the fully routed reviewed-server
  revocation proof to the existing non-copyable responder-path evidence type.
- Preserve every routed signature choice and exact-observation validity token
  without new validation or effects, while leaving routed responder DNS,
  no-check, revocation, TLS, HTTP, and OAuth readiness unresolved.
- Add a distinct binder from the fully routed OCSP proof to the routed
  constructed reviewed-server path, preserving response, responder-certificate,
  target-certificate, and both server-path signature routes.
- Retain exact target/intermediate identity and observation-time validity with
  stable payload-free failures, no ambient capabilities or external effects,
  and no responder-path, TLS, HTTP, or OAuth-readiness claim.
- Add a distinct binder from the routed RSA SHA-256/SHA-384/SHA-512 OCSP
  response-signature proof to the routed constructed reviewed-server path.
- Retain both complete proofs and exact-observation target/intermediate
  validity without implying routed OCSP certificate links, routed responder
  policy, external effects, or transport readiness.
- Bind the authenticated OCSP target-revocation proof to the data-driven routed
  constructed reviewed-server path only when both retain the same exact target
  and intermediate certificate objects.
- Retain both complete non-copyable proofs plus fresh target/intermediate
  validity at the OCSP observation time, with stable payload-free failures and
  without claiming routed responder policy, TLS, HTTP, or OAuth readiness.
- Require and semantically validate exact DER `NULL` for the authenticated
  responder's RFC 6960 `id-pkix-ocsp-nocheck` extension after reviewed-path and
  responder DNS policy, without allowing certificate or proof substitution.
- Retain the complete non-copyable reviewed responder proof and expose only
  payload-free missing or malformed no-check failures, while leaving responder
  certificates without no-check and intermediate revocation unresolved.
- Apply the exact retained intermediate's supported critical permitted-and-
  excluded-DNS constraints to the authenticated responder certificate, without
  allowing responder, SAN, intermediate, or path substitution.
- Retain count-only intermediate DNS evidence with payload-free failures while
  leaving excluded anchor constraints, non-DNS constraints,
  responder/intermediate revocation, generalized path validation, and
  transport unresolved.
- Apply the selected reviewed root's supported out-of-band DNS constraints to
  the exact authenticated responder certificate retained by the reified path,
  under separate caller-owned parse limits and with payload-free failures.
- Retain count-only DNS evidence while leaving intermediate, excluded, and
  non-DNS responder constraints and responder/intermediate revocation unresolved.
- Add an infallible nominal projection that retains the complete reviewed
  server-revocation proof and reifies its already-proven exact delegated
  responder-to-reviewed-root path for later policy composition.
- Preserve responder and intermediate exact-observation validity without new
  cryptography or effects, while leaving responder name constraints and
  responder/intermediate revocation explicitly unresolved.

## 0.1.0

- Add exact target-and-issuer binding between authenticated OCSP revocation and
  a constructed, Mozilla-reviewed, anchor- and intermediate-name-constrained
  RSA SHA-256 server path.
- Retain both complete non-copyable inputs and expose only stable payload-free
  target/issuer mismatch categories.
- Revalidate and retain target and intermediate validity at the exact OCSP
  observation time, rejecting cross-time evidence mixing with stable redacted
  target/issuer validity categories.
- Add a static public Apple/DigiCert stapled-OCSP integration fixture with
  recorded acquisition and verification provenance.
