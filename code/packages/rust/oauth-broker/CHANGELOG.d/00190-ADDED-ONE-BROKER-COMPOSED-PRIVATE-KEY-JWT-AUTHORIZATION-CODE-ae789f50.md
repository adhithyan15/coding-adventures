- Added one broker-composed `private_key_jwt` authorization-code exchange
  boundary that validates the already callback-consumed request against the
  registered provider, client ID, token endpoint, retained method, and exact
  advertised algorithm before the existing audited abstract signer, then
  audit-brackets injected transport and bounded response decoding. Issued-at
  time, replay entropy, signing authority, and transport remain caller-owned;
  no concrete algorithm, persistence, or network authority is enabled.
