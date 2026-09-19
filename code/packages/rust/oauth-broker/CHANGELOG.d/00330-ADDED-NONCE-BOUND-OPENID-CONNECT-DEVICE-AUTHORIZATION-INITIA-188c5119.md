- Added nonce-bound OpenID Connect device authorization initiation. The broker
  checks the registered profile before entropy or transport, requires exact
  `openid`, sends an independent 256-bit nonce through the existing audited
  device transport, and retains zeroizing provider/client/trace-bound nonce
  state beside the validated verification and polling state. Identity proof,
  account-key selection, storage, timing, and concrete transport remain out of
  scope.
