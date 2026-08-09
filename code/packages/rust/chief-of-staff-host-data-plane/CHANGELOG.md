# Changelog

## Unreleased

- Expose exact model-registry cardinality without exposing provider clients.
- Document production daemon injection for non-empty typed authority declarations.
- Add an exact zeroizing pipeline/agent/channel key registry for safe production
  provisioning adapters, with fail-closed duplicate, direction, and identity scope.

- Add the authority-backed concrete service over real encrypted durable channel
  endpoints and exact provider-neutral LLM clients.
- Retain a bounded receive-to-ack delivery ledger, provision sealed receiver
  grants before publication, and fail closed on unknown keys or model selectors.
- Validate all channel/provider response fields against the authenticated wire
  bounds before returning them to process supervision.
- Add durable per-request pipeline authorization, injected service dispatch,
  response-shape validation, and a fail-closed unavailable production service.
