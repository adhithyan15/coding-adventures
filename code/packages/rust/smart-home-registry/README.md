# smart-home-registry

In-memory smart-home registry for normalized bridge, device, entity, scene,
state, event, and protocol mappings.

This crate is the first D23 registry implementation slice. It is intentionally
pure and in-memory so protocol adapters can start sharing identity and lookup
semantics before durable D18A storage, Vault, actor supervision, or real device
I/O are added.

## Scope

Current scope:

- bridge/device/entity/scene upserts
- bridge-to-device and device-to-entity indexes
- protocol-native identifier lookup
- selector-based device/entity queries
- state freshness helpers for stale or missing cached state
- refresh plans for missing or stale entity state by bridge/device identity
- refresh-result application with refreshed/missing entity reports
- state snapshot cache
- immutable event log in arrival order
- state updates from normalized event deltas
- conflict detection for duplicate protocol identifiers

Out of scope:

- durable storage backends
- Vault reference resolution
- command routing
- actor/event-bus supervision
- Hue, Zigbee, Z-Wave, Thread, or Matter I/O

## Dependencies

- `smart-home-core`

## Development

```bash
bash BUILD
```
