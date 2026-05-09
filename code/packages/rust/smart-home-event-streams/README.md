# smart-home-event-streams

Pure event-stream cursor and supervision primitives for D23 smart-home
integrations.

This crate does not open sockets, subscribe to MQTT topics, call cloud APIs, or
parse vendor payloads. It gives Hue SSE, WebSocket, MQTT, cloud-push, serial, and
radio workers a shared deterministic shape for:

- stream identity and transport classification
- stream cursors and replay checkpoints
- heartbeat freshness and stale-event deadlines
- disconnect tracking without losing the last cursor
- reconnect attempts with bounded exponential backoff
- restart plans that a runtime supervisor can inspect before spawning workers

Protocol-specific transport clients, payload parsers, and adapter actors live in
integration crates. This crate owns the boring rules that should stay the same
across all of them.

## Dependencies

- smart-home-core

## Development

```bash
bash BUILD
```
