- Added one composed client-secret authorization-code exchange-to-custody
  boundary: an exact provider-bound opaque account key is required before any
  secret, transport, clock, or credential-store access; the decoded response
  crosses existing OAuth release and custody-create audits without credential
  disclosure, and only the opaque revision leaves the broker.
