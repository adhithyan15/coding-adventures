# Changelog

## Unreleased

- Exercise the real child with durable launch bindings and the daemon's production
  data-plane composition: owner-only channel keys, encrypted input/output stores,
  an explicit Ollama provider, publication, and acknowledgement all run together.
- Add the concrete Level 1 host executable with independent package verification,
  exact authenticated launch-policy matching, serialized channel/model requests,
  bounded idle polling, heartbeats, and graceful authenticated termination.
- Back off and retry only read-only receive unavailability; keep failures after
  input delivery terminal to avoid unsafe completion or publication retries.
- Fail closed unless the first Level 1 production topology has exactly one read
  channel and one write channel.
