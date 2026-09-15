- Added exact stored private-key-JWT refresh composition: the complete retained
  profile is validated before the selected credential's refresh token is
  released; signing and transport remain injected and audited; and the bounded
  response is atomically retained or rotated only at the loaded revision. All
  failures preserve the prior credential.
