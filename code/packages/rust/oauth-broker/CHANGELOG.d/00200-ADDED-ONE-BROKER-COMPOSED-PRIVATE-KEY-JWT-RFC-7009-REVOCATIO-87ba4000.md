- Added one broker-composed `private_key_jwt` RFC 7009 revocation boundary:
  retained provider policy now derives a separate assertion profile only from
  an explicitly configured revocation audience, and provider, client,
  endpoint, method, and exact algorithm checks precede the audited abstract
  signer. Injected transport and core exact-200 response classification are
  separately audit-gated; no local credential is deleted and no concrete
  algorithm or network authority is enabled.
