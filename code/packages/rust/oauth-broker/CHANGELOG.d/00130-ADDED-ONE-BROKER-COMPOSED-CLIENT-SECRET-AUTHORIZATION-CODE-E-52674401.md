- Added one broker-composed client-secret authorization-code exchange boundary
  that checks the registered provider, client ID, token endpoint, and retained
  Basic/Post method before audited secret access, then audit-brackets injected
  transport and bounded response decoding without adding browser, persistence,
  or concrete network authority.
