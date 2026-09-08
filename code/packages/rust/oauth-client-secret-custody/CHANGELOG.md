# Changelog

## Unreleased

- Added provider-driven `client_secret_basic` and `client_secret_post`
  authentication for authorization-code exchange, refresh, and revocation
  requests. Secret access stays inside the audited custody closure, wire
  credentials are zeroizing, and every request retains exact provider, client,
  and trace binding.
- Added opaque client-secret references, zeroizing secret ownership, an
  injected revision-bound CAS store, durable audit-before-access/edit, closed
  diagnostics, and a deterministic in-memory reference store.
