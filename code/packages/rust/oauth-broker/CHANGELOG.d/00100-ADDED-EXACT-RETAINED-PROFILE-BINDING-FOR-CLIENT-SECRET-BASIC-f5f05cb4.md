- Added exact retained-profile binding for `client_secret_basic` and
  `client_secret_post` into the existing opaque client-secret adapter, with
  provider mismatch and public/private-key profiles rejected before custody
  access and no caller-selected authentication method.
