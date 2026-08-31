# Changelog

## Unreleased

- Exposed read-only provider, trace, and redirect-URI ceremony bindings on
  `AuthorizationRequest` so an authorized host can reject cross-ceremony or
  cross-provider browser release without exposing transaction secrets.
- Added provider-neutral RFC 8414 request preparation and bounded metadata
  decoding with exact issuer comparison, strict HTTPS endpoints, explicit
  Authorization Code, public-client `none`, PKCE `S256`, and RFC 9207 response
  issuer or registry-owned distinct-redirect negotiation,
  immutable capability retention, provider-config derivation, recursive JSON
  scrubbing, and mandatory audit publication before either request or response
  release.
- Added provider-neutral bounded JSON/form token response decoding, closed
  token-endpoint errors, explicit refresh-token rotation decisions, audited
  public-client refresh and RFC 7009 revocation request preparation, recursive
  response-tree scrubbing, and a separate audit gate before parsed credential
  material can leave the codec. Secret form encoders now write directly into a
  zeroizing destination without ordinary heap-string intermediates.
- Added the first provider-neutral OAuth 2.0 installed-app primitive: strict
  configuration, caller-injected 256-bit state and PKCE entropy, mandatory
  `S256`, authorization URL construction, exact callback/state/issuer
  validation, opaque token-exchange preparation, closed errors, redacted
  diagnostics, caller-owned trace correlation, and first-class privacy-safe
  audit descriptors whose durable publication is required before result
  release.
