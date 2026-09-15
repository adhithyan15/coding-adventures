- Added exact stored client-secret refresh composition: retained provider
  authentication is validated before the selected credential's refresh token
  is released, and an audit-gated bounded response is atomically retained or
  rotated only at the loaded revision. All failure paths retain prior state.
