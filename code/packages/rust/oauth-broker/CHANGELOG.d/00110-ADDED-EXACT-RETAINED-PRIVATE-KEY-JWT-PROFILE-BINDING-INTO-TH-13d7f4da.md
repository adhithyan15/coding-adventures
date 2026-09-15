- Added exact retained `private_key_jwt` profile binding into the existing
  opaque-key assertion adapter, including provider, client ID, token-endpoint
  audience, and advertised algorithm checks without invoking a signer or
  enabling any concrete algorithm.
