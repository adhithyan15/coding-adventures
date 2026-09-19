- Added proof-derived public Authorization Code exchange custody. Exact
  identity-profile, provider, client, nonce, and trace binding now precedes
  injected transport and clock access; the response ID token is detached and
  consumed by the audited identity authority, and only its opaque account key
  selects credential creation. Missing or rejected evidence reaches no
  credential store, the ID token is not retained, and only the opaque revision
  is released. No concrete verifier, algorithm, network, clock, storage,
  provider data, or external dependency is added.
