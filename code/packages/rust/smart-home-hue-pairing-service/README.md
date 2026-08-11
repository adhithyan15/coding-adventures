# smart-home-hue-pairing-service

Actor-owned Hue bridge pairing over real LAN HTTP/TLS with recoverable durable
credential installation.

The service receives the shared `SmartHomeControllerRuntime` authority and
resolves every pending `smart-home-pairing-transaction` journal before it can
accept a request. Each schema-v2 request carries the exact D23 principal and
expected durable runtime revision. Authorization is checked before LAN or
secret-storage activity, and the coordinator then seals the returned
application key, commits through the exact central revision, and cleans any
captured previous credential at its exact revision. Successful completion is
immediately visible through the controller's discovery, automation, and Home
Assistant-compatible runtime handles; the actor keeps no private runtime copy.

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
