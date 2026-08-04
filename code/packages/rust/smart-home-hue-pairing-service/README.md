# smart-home-hue-pairing-service

Actor-owned Hue bridge pairing over real LAN HTTP/TLS with credentials written
to the repository's sealed Vault store.

The service consumes a pending D23 pairing session, builds the canonical Hue
registration request, executes it through an injectable transport, seals the
returned application key, and completes the runtime session with only an
opaque `VaultRef`. Runtime state, events, actor snapshots, and reports never
contain the raw Hue credentials.

`HueLanRegistrationTransport` supports bounded HTTP/1 over local TCP and TLS.
Production HTTPS callers provide the Hue trust root through `TlsConfig`; the
transport does not silently disable certificate verification.

## Development

```bash
bash BUILD
```
