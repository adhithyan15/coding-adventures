- Added one composed `private_key_jwt` authorization-code exchange-to-custody
  boundary: an exact provider-bound opaque account key is required before
  signing, transport, clock, or credential-store access; the bounded response
  crosses existing OAuth release and custody-create audits without credential
  disclosure, and only the opaque revision leaves the broker.
