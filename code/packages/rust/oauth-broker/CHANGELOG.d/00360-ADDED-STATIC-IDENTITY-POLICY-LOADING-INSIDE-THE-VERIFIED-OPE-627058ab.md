- Added static identity-policy loading inside the verified OpenID Connect
  device credential composition. The opaque verification context, authorized
  response, nonce, registered deployment client, provider, and trace are bound
  before policy-source or later authority access; policy loading, clock,
  credential release, identity proof, custody, and broker results keep separate
  audit gates. The caller supplies no decoded identity policy, and no concrete
  source, verifier, algorithm, clock, storage, transport, or dependency is
  added.
