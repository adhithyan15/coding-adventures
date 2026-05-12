# smart-home-local-http

Pure local HTTP request planning primitives for D23 smart-home integrations.

This crate does not open sockets, perform DNS, validate certificates, execute
retries, or resolve secrets. It gives Hue, Shelly, WLED, ESPHome, camera, and
energy-gateway workers a shared deterministic shape for:

- local bridge endpoint identity
- scheme, port, base-path, and TLS policy
- request method, timeout, idempotency, and media-type hints
- bounded retry and backoff policy metadata for supervised local calls
- vault-backed auth placeholders without exposing secret values
- header conflict checks before a runtime worker receives the plan
- aggregate request-plan summaries for method, auth, retry, body, and timeout
  shape before execution
- bounded read-side queries for endpoint inventories and planned requests

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
