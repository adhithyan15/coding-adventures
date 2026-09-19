- Added one caller-timed OpenID Connect device step that preserves the opaque nonce-bound polling sequence across continuation outcomes and, on authorization, loads static identity policy, verifies the ID token, and stores credentials only under the derived opaque key.
  The verification context is provider-bound before polling; continuation
  outcomes reach no policy, wall-clock, verifier, or credential authority;
  every existing transport, policy-load, proof, credential-release, custody,
  and broker audit gate remains separate. No concrete authority, algorithm,
  provider data, network implementation, or dependency is added.
