# Changelog

## 0.1.0

- Add a provider/trace-audited identity binder for one custody-built OAuth
  client-secret request that constructs its exact fixed body, canonical
  Content-Length proof, and method-selected optional Authorization field only
  after the attempted audit is durable.
- Prevent callers from supplying substituted credential bytes or prebuilt
  evidence to the binder, leave bounded caller storage unchanged on validation
  or audit failure, and perform no credential read after terminal success.
- Keep stable failures, audit events, and Debug output free of endpoints,
  credentials, body bytes, field bytes, lengths, and other sensitive payloads.
- Perform no credential storage access, encoding, signing, transport, TLS,
  HTTP I/O, socket access, or live-provider authorization.
