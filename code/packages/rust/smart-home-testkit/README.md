# smart-home-testkit

Deterministic fixtures and fake streams for D23 smart-home runtime and
integration tests.

This crate does not open sockets, touch radios, read files, or call cloud APIs.
It gives future Hue, MQTT, Zigbee, Z-Wave, Thread, Matter, and runtime packages
a shared way to build:

- normalized bridge/device/entity fixtures
- confirmed, stale, and optimistic state snapshots
- deterministic device events
- scripted fake event streams with disconnect and gap markers
- simple logical clocks for freshness and supervision tests

## Dependencies

- `smart-home-core`

## Development

```bash
bash BUILD
```
