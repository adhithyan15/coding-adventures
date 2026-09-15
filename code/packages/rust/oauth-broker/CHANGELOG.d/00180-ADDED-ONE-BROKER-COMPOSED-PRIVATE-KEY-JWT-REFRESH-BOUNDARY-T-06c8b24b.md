- Added one broker-composed `private_key_jwt` refresh boundary that validates
  the registered provider, client ID, token endpoint, retained method, and
  exact advertised algorithm before the existing audited abstract signer,
  then audit-brackets injected transport and bounded response decoding. Time,
  replay entropy, signing authority, and transport remain caller-owned; no
  concrete algorithm or network authority is enabled.
