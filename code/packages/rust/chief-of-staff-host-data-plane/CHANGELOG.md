# Changelog

## Unreleased

- Add `CompositeModelToolDispatcher`, several `ModelToolDispatcher`s behind one,
  routed by tool name. The service composed exactly one dispatcher, which is
  what made smart home *the* model tool surface rather than one agent among
  several: there was no way to add a second source without replacing the first.
- Two sources claiming one tool name is an error on both the offer and the
  execute path, not a first-match win. Silently picking one would mean the tool
  a model called and the tool that ran could differ by nothing but construction
  order.

- Expose the exact binding-aware installed D18D catalog through a separately
  authorized data-plane operation while preserving exact-catalog enforcement
  on every later tool completion.
- Add an injected manifest-blind model-tool dispatcher, require the entire
  offered catalog to equal its installed catalog, and carry exact structured
  D18D execution results back to the authenticated child.
- Carry bounded tool-aware completion turns through exact-model authorization,
  adapt declarations and prior results to `llm-gateway`, and retain structured
  calls, provider identity, usage, finish reason, latency, and polyfill evidence.
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
