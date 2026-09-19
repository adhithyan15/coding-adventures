- Added verified OpenID Connect device credential creation through the existing
  audited identity and custody authorities. Exact provider, client, response,
  nonce, profile, and trace binding precedes clock access and credential
  release; the ID token is consumed by proof, remaining credentials are stored
  only under the derived opaque account key, and only the revision is returned.
  Missing or rejected identity evidence reaches no credential store. No
  concrete policy loader, verifier, algorithm, clock, storage, transport, or
  external dependency is added.
