- Added public static identity-policy Authorization Code exchange custody.
  Registered public profile, client, endpoint, opaque verification context,
  nonce, and trace bindings now fail before policy-source or effect access. The
  exact static profile then crosses separately audited source, transport,
  identity-proof, credential-release, and opaque-key custody gates. Only the
  credential revision leaves the broker; no concrete source, verifier, network,
  clock, storage, provider data, algorithm, or external dependency is added.
