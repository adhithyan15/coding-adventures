- Added one broker-composed client-secret RFC 7009 revocation boundary that
  checks the registered provider, client ID, revocation endpoint, and retained
  Basic/Post method before audited secret access, then audit-brackets an
  injected bounded transport and core response classification. Exact HTTP 200
  confirms revocation; retryable and provider failures never imply local
  credential deletion.
