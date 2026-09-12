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
The returned assertion retains provider, client, audience, and trace bindings
for the request-binding layer to verify before HTTPS transport.

This package implements no signing algorithm and grants no network authority.
An injected HSM or operating-system signer can satisfy the existing boundary.
Repository-owned Ed25519 must be made constant-time and zeroizing before it can
serve as an `EdDSA` authority, and `RS256` still requires repository-owned RSA.
