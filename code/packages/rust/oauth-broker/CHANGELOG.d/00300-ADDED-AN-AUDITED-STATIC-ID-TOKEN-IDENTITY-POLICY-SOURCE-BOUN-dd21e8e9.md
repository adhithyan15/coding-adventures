- Added an audited static ID-token identity-policy source boundary. Existing
  provider registration, deployment client ID, requested provider, opaque
  verification context, and trace are exactly bound before the source read;
  zeroizing bytes are decoded through the closed identity schema and the
  profile is withheld unless its result audit is durable. No concrete source,
  verifier, key, clock, storage, or network authority is added.
