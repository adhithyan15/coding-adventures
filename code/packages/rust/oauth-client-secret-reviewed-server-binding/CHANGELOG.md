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
- Restricted the direct-revocation marker to completions whose retained server
  status proof includes the exact signed reviewed path-and-anchor-self-
  signature proof shape.
- Kept the direct revocation proof additive to the signed path instead of
  admitting it as a substitute for signed reviewed-server evidence.
