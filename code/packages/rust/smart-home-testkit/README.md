# smart-home-testkit

Deterministic fixtures and fake streams for D23 smart-home runtime and
integration tests.

This crate does not open sockets, touch radios, read files, or call cloud APIs.
It gives future Hue, MQTT, Zigbee, Z-Wave, Thread, Matter, and runtime packages
a shared way to build:

- normalized bridge/device/entity fixtures
- registry seeding helpers for installing fixture records into
  `smart-home-registry`
- confirmed, stale, and optimistic state snapshots
- deterministic device events
- scripted fake event streams with disconnect and gap markers
- Hue SSE event-stream specs, connected stream states, and drivers that apply
  scripts to both event-stream supervision state and `smart-home-runtime`
- fake command buses with queued command/result pairs
- fake MQTT broker publications with retained-message and metadata markers
- read-only fake MQTT broker queries by topic, prefix, retained flag, metadata,
  observation time, sort, and limit
- simple logical clocks for freshness and supervision tests

## Dependencies

- `smart-home-core`
- `smart-home-event-streams`
- `smart-home-registry`
- `smart-home-runtime`

## Development

```bash
bash BUILD
```
