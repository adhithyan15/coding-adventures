# Changelog

## Unreleased

- Run a bounded Level 1 model/tool loop: discover the binding-authorized catalog,
  execute each exact model call only through parent-owned D18D authority, require
  correlated results, replay them to the model, and publish only final text. Cap
  the loop at eight model turns and preserve text-only behavior only when catalog
  discovery is explicitly unavailable.
- Override `LlmClient::complete_with_tools` with the authenticated child-control
  transport and preserve structured calls/results plus provider audit metadata.
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
