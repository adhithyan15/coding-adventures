# OAuth private-key JWT assertions

This crate constructs bounded RFC 7523 OAuth client assertions through the
repository's audit-first, non-exporting private-key signer. Provider identity,
client identity, token-endpoint audience, authentication method, signing
algorithm, opaque key reference, lifetime, and optional `kid` are validated as
data before signing.

Each assertion contains exact `iss`, `sub`, `aud`, `exp`, `iat`, and `jti`
claims. The caller supplies 32 bytes from a cryptographically secure random
source for `jti`; the crate owns no entropy or clock authority. JSON and Base64
encoding use repository-owned primitives, while signing input, signature
encoding, and the final assertion are wipe-on-drop values.

The only effect is delegated to `oauth-private-key-signer`, whose durable
attempt/result events contain the provider, opaque key, exact algorithm, and
caller trace. Pure assertion construction needs no additional audit event.
The profile can now authenticate already audit-released authorization-code,
refresh-token, and revocation requests. Provider, client, endpoint audience,
and trace are bound before signing; a mismatch prevents both signer audit and
the signing effect. The result carries a zeroizing form body with exact
`client_assertion_type` and `client_assertion` fields, while exchange and
refresh results preserve their response-decoder context. Revocation requires a
profile whose audience exactly matches the revocation endpoint rather than
assuming that a provider accepts its token endpoint as that audience.

Transport remains injected and must independently audit its external effect
with the retained provider and trace. The request-binding layer adds no network
or storage authority and emits no duplicate event for its pure form assembly.

This package implements no signing algorithm and grants no network authority.
An injected HSM or operating-system signer can satisfy the existing boundary.
Repository-owned Ed25519 must be made constant-time and zeroizing before it can
serve as an `EdDSA` authority, and `RS256` still requires repository-owned RSA.
