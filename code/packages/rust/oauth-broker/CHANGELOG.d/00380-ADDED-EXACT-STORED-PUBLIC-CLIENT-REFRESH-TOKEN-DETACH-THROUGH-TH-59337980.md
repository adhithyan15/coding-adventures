- Added exact stored public-client refresh-token detach through the retained `none` profile, audited injected RFC 7009 transport, exact HTTP 200 classification, and revision-bound conditional credential deletion.
  Provider profile and endpoint validation precede credential access; missing
  refresh data, HTTP 503, transport and protocol failures, and stale revision
  conflicts retain local credentials. The boundary adds no access-token
  fallback, concrete transport, provider data, or dependency.
