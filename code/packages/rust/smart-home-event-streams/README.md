# smart-home-event-streams

Pure event-stream cursor and supervision primitives for D23 smart-home
integrations.

This crate does not open sockets, subscribe to MQTT topics, call cloud APIs, or
parse vendor payloads. It gives Hue SSE, WebSocket, MQTT, cloud-push, serial, and
radio workers a shared deterministic shape for:

- stream identity and transport classification
- MQTT topic filter validation, matching, and subscription descriptors
- MQTT publication descriptors for outbound command topics and audit metadata
- Home Assistant MQTT discovery topic planning for config, state,
  availability, and command surfaces
- stream cursors and replay checkpoints
- checkpoint-based state resume after supervised worker restarts
- heartbeat freshness and stale-event deadlines
- heartbeat deadline schedules for supervisor wakeups across many streams
- disconnect tracking without losing the last cursor
- reconnect attempts with bounded exponential backoff
- restart schedules that group due reconnect plans across stream workers
- restart plans that a runtime supervisor can inspect before spawning workers
- deterministic state queries for dashboards, supervisors, and read-only tools
- compact fleet summaries for supervisor health and restart-readiness checks
- transport-family counts in fleet summaries for supervisor coverage views

Protocol-specific transport clients, payload parsers, and adapter actors live in
integration crates. This crate owns the boring rules that should stay the same
across all of them.

## Dependencies

- smart-home-core

## Development

```bash
bash BUILD
```
