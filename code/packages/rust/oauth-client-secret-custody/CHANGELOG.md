# Changelog

## Unreleased

- Retained the exact provider/trace response context across authenticated RFC
  7009 request construction so a later broker transport cannot classify an
  unbound response.
- Exposed the selected client-secret method's shared confidential metadata
  method so composition roots can derive an exactly matching provider config.
- Added exact, case-sensitive RFC 8414 method selection for provider-bound
  client-secret authentication, rejecting unadvertised methods and
  cross-provider opaque keys before any credential access.
- Added provider-driven `client_secret_basic` and `client_secret_post`
  authentication for authorization-code exchange, refresh, and revocation
  requests. Secret access stays inside the audited custody closure, wire
  credentials are zeroizing, and every request retains exact provider, client,
  and trace binding.
- Added opaque client-secret references, zeroizing secret ownership, an
  injected revision-bound CAS store, durable audit-before-access/edit, closed
  diagnostics, and a deterministic in-memory reference store.
