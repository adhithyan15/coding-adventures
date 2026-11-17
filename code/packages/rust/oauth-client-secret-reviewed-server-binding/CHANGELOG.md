# Changelog

## Unreleased

- Added a sealed identity-only composition between one complete audited OAuth
  client-secret HTTP/1 request and one signed reviewed HTTPS server path.
- Required exact decoded-certificate object identity plus equal SNI and Host
  identities, including the Host port, before retaining both complete proofs.
- Kept request-byte access, audit, revocation, TLS, HTTP, I/O, and every
  external effect outside this boundary.
- Added a second sealed composition that consumes the complete request/signed-
  server binding and the destination-bound direct no-check complete fixed-path
  revocation proof.
- Required the signed-server and revocation proofs to retain the same exact
  leaf, target-nearest issuer, reviewed anchor, SNI, Host, and optional port
  before retaining both complete inputs.
- Required exactly one processed intermediate on both retained paths so the
  direct branch cannot overstate its target-plus-intermediate revocation
  coverage for a longer path.
- Restricted the direct-revocation marker to completions whose retained server
  status proof includes the exact signed reviewed path-and-anchor-self-
  signature proof shape.
- Kept the direct revocation proof additive to the signed path instead of
  admitting it as a substitute for signed reviewed-server evidence.
- Added a distinct sealed composition for the destination-bound nested
  no-check complete fixed-path revocation branch.
- Restricted that nested marker to the exact retained signed reviewed path-
  and-anchor-self-signature proof shape beneath its responder-revocation and
  responder-no-check layers.
- Repeated the exact leaf, issuer, anchor, SNI, Host, port, and one-intermediate
  checks because the additional responder layers do not extend the nested
  branch's target-plus-intermediate server-chain coverage.
- Added a distinct sealed composition for the destination-bound double-nested
  no-check complete fixed-path revocation branch.
- Restricted that marker to the exact signed reviewed path-and-anchor-self-
  signature proof retained beneath both responder-revocation layers.
- Repeated the exact leaf, issuer, anchor, SNI, Host, port, and one-intermediate
  checks because the additional responder paths do not extend the double-
  nested branch's target-plus-intermediate server-chain coverage.
- Left recursive and alternative revocation branches outside this boundary
  and made no TLS, HTTP, network, or live-provider readiness claim.
