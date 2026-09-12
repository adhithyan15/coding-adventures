# Changelog

## Unreleased

- Added bounded RFC 7523 `private_key_jwt` assertion construction with exact
  provider capability checks and caller-supplied replay entropy.
- Kept JWS input, signatures, and complete assertions in wipe-on-drop storage
  while delegating the only signing effect to the audited non-exporting signer.
