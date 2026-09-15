- Added exact access-token-only detach through retained client-secret policy:
  custody refuses the fallback when a refresh token exists, retryable and
  closed failures retain the record, and only exact HTTP 200 permits deletion
  of the revision released with the access token.
