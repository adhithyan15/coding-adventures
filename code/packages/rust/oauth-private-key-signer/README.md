# OAuth private-key signer

This crate is the non-exporting signing prerequisite for OAuth
`private_key_jwt` client authentication. Provider configuration retains an
opaque key reference; private-key bytes live only inside an injected signing
authority and never cross the signing API.

Every signature is bracketed by durable, privacy-safe audit events containing
the provider, opaque key reference, selected algorithm, caller-owned trace,
action, and closed outcome. Audit failure before an operation prevents it.
Audit failure after an operation withholds the result.

Algorithms remain validated provider data and `none` is rejected. This crate
does not claim a concrete production software algorithm: the repository's
current Ed25519 scalar operations are not constant-time, so they must be
hardened before they may hold OAuth signing authority. `RS256` likewise needs a
separate repository-owned RSA primitive. Injected HSM or operating-system
authorities can implement the same non-exporting contract without widening the
OAuth protocol core.

JWT serialization, claims, replay-resistant `jti` generation, and request
binding intentionally remain outside this package and will consume this
authority in the next OAuth slice.
