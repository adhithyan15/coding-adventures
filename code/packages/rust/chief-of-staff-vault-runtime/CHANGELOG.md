# Changelog

## Unreleased

- Add a replaceable trusted direct-delivery boundary that transfers zeroizing
  secret payloads to approved consumers while exposing only success or bounded,
  secret-free failures to callers.
- Define the opaque `{ vault_ref, expires_at_ms }` receipt as the canonical
  agent-facing lease contract and redact the bearer reference from `Debug` output.
