- Added static identity-policy proof composition. The broker derives the trace
  only from the one-use authorization nonce, validates the registered provider,
  deployment client, and opaque verification context before source access,
  then carries the zeroizing ID token through the existing separately audited
  policy-load and trusted-verification gates. Only the verified provider-scoped
  opaque credential key is released after the composite result audit; no
  concrete source, verifier, JWT/JOSE/JWKS algorithm, clock, storage, transport,
  or network authority is added.
