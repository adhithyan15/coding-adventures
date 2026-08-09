# smart-home-hue-pairing-service

Actor-owned Hue bridge pairing over real LAN HTTP/TLS with recoverable durable
credential installation.

The service restores its actor runtime from `smart-home-runtime-store` and
resolves every pending `smart-home-pairing-transaction` journal before it can
accept a request. Each schema-v2 request carries the exact D23 principal and
expected durable runtime revision. Authorization is checked before LAN or
secret-storage activity, and the coordinator then seals the returned
application key, commits the complete runtime snapshot with CAS, cleans any
captured previous credential at its exact revision, and returns the committed
snapshot for actor-state replacement.

Raw application and client keys remain in zeroizing process-local values and
the sealed Vault payload. Runtime state, journals, events, actor snapshots,
messages, and reports contain only the opaque `VaultRef` and non-secret
metadata. Startup fails closed when journal recovery or replacement cleanup
cannot complete safely.

`HueLanRegistrationTransport` supports bounded HTTP/1 over local TCP and TLS.
Production HTTPS callers provide the Hue trust root through `TlsConfig`; the
transport does not silently disable certificate verification.

## Development

```bash
bash BUILD
```
