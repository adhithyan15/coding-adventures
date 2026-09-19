- Added nonce-retaining OpenID Connect device polling that keeps the
  provider/client/trace-bound nonce inside one opaque caller-timed sequence
  through waits, pending responses, slow-down, and retryable transport
  failures. Authorization releases only a nonce-paired audited token response
  for later identity proof; denial and expiry discard the nonce. No verifier,
  account-key selection, storage, clock, sleep, or concrete transport is added.
