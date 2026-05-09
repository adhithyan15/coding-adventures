# smart-home-local-http

Pure local HTTP request planning primitives for D23 smart-home integrations.

This crate does not open sockets, perform DNS, validate certificates, retry
requests, or resolve secrets. It gives Hue, Shelly, WLED, ESPHome, camera, and
energy-gateway workers a shared deterministic shape for:

- local bridge endpoint identity
- scheme, port, base-path, and TLS policy
- request method, timeout, idempotency, and media-type hints
- vault-backed auth placeholders without exposing secret values
- header conflict checks before a runtime worker receives the plan

Protocol-specific clients and runtime executors live in integration crates. This
crate owns the portable request vocabulary that should stay the same across
them.

## Dependencies

- http-core
- smart-home-core

## Development

```bash
bash BUILD
```
