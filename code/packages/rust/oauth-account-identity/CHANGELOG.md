# Changelog

## Unreleased

- Added bounded exact-schema static ID-token identity-provider policy decoding.
  Provider, HTTPS issuer, and case-sensitive non-`none` algorithm data are
  retained while deployment client ID and opaque verification context remain
  caller-owned; decoding adds no source, verifier, crypto, clock, or network
  authority.
- Added an audit-first injected authority for deriving provider-scoped opaque
  credential keys from bounded zeroizing ID-token evidence.
- Bound verification to exact provider, client, issuer, nonce, caller time,
  allowed algorithm data, opaque verification context, and caller trace.
- Added Authorization Code verification that consumes the non-cloneable OAuth
  nonce binding and derives provider, client, and trace from that ceremony
  before any identity-authority effect.
- Kept JWT, JOSE, JWKS, discovery, clock, network, storage, and concrete
  cryptographic authority outside this package.
