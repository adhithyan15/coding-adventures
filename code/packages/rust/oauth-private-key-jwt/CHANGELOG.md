# Changelog

## Unreleased

- Added bounded RFC 7523 `private_key_jwt` assertion construction with exact
  provider capability checks and caller-supplied replay entropy.
- Kept JWS input, signatures, and complete assertions in wipe-on-drop storage
  while delegating the only signing effect to the audited non-exporting signer.
- Bound signed assertions into authorization-code exchange, refresh, and
  revocation form requests only after exact provider/client/audience checks,
  preserving provider/trace response and transport context.
- Zeroized caller-supplied replay entropy on every local exit path, including
  invalid-time and pre-signing request-binding failures.
