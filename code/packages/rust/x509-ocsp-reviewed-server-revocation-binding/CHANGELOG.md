# Changelog

## Unreleased

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
