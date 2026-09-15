- Added one exact refresh-token detach composition: the registered provider and
  retained client-secret method are validated before credential access, the
  refresh token and revision come from the selected opaque account record, and
  conditional local deletion occurs only after an audit-gated exact HTTP 200
  revocation response. All closed failure paths retain the credential record.
