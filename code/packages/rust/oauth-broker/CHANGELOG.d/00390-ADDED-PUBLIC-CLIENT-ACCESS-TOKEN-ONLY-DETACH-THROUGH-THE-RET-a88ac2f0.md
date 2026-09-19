- Added public-client access-token-only detach through the retained `none`
  profile, custody proof that no refresh token exists, audited injected RFC 7009
  transport, exact HTTP 200 classification, and revision-bound conditional
  deletion. Refreshable credentials, confidential-profile mismatch, provider or
  transport failure, and stale revisions retain local credentials.
