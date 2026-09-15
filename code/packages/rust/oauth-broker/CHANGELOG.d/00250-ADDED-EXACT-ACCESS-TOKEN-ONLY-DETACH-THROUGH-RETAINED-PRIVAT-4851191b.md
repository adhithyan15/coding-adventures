- Added exact access-token-only detach through retained `private_key_jwt`
  policy: custody refuses refreshable records before signing or transport,
  while retryable and closed failures retain access-only records and exact HTTP
  200 permits deletion only after every signer, transport, protocol, custody,
  and broker audit gate.
