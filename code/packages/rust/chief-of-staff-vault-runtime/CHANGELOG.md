# Changelog

## Unreleased

- **Breaking**: `VaultDirectDelivery::deliver` and
  `ChiefVaultRuntime::request_direct` now take a `VaultDirectRequest` carrying
  the requesting agent, the session, and the secret name alongside the
  destination, instead of a bare `consumer_agent_id`. An adapter told only the
  destination cannot authorize anything: the strongest rule it can express is a
  global destination allowlist, under which a caller cleared to send one secret
  to a consumer is equally cleared to send every secret to it, because nothing
  in the chain can tell the two requests apart. That is a confused deputy — the
  adapter holds the authority but not the facts. Naming the requester and the
  secret does not authorize anything by itself; it is the precondition for an
  adapter that wants to.

- Add a replaceable trusted direct-delivery boundary that transfers zeroizing
  secret payloads to approved consumers while exposing only success or bounded,
  secret-free failures to callers.
- Define the opaque `{ vault_ref, expires_at_ms }` receipt as the canonical
  agent-facing lease contract and redact the bearer reference from `Debug` output.
