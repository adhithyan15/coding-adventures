# Changelog

## Unreleased

- Add `open_delivered_message` (D18S P2.6d): every per-message check a
  receiver makes, with no storage access. Grants come through a callback,
  once per epoch. `DurableReceiver` uses it, so a broker holding the
  receiver key runs exactly the same checks.
- Add the D18P compatibility adapter plus a deterministic shared durable-channel
  fixture generator and byte-identical Rust conformance consumer.
- Consume authenticated channel messages through the channel-crypto package's
  structurally immutable read-only API.

## 0.1.0

- Add create-if-absent durable channel membership definitions.
- Enforce one originator, a disjoint receiver set, bound public keys, and an
  active/destroyed lifecycle.
- Add authorized originator and receiver traits plus durable endpoint facades.
- Add encrypted publish, sealed-grant distribution, verified receive, and
  message-ID acknowledgement flows.
