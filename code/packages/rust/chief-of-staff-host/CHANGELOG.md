# Changelog

## Unreleased

- **P1.5: the weather reference agent** (#142, #13980). It lives at
  `reference-agents/weather/SKILL.md`: one SKILL.md file and no code, Tier 1,
  asking for `net.fetch` and the `api.weather.gov` network capabilities.
  - The new end-to-end test `tests/weather_reference_agent.rs` signs it and
    registers it, and the real process supervisor spawns the real host binary
    from it.
  - It sends "Seattle" on a sealed channel, and the host asks a scripted
    Ollama model.
  - The model calls `net.fetch`. The production daemon data plane offers the
    tool from the signed manifest and executes it, with only DNS and the TLS
    transport faked.
  - The answer arrives on the report channel. The test also checks the exact
    request on the wire, and that headers off the allowlist never reach the
    model.
- The production-composition test writes the developer public key its
  config names. Composing a model-tool surface now loads the keyring, because
  the daemon's agent tool source verifies each host's package before offering
  `net.fetch` or `vault.request_lease`.
- Extend the production subprocess gate through a real Ollama-selected
  `smart_home.list_devices` call, signed-host authorization, central durable D23
  audit commit, result replay, and Home Assistant-compatible entity/audit
  readback after reopening the controller state.
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
