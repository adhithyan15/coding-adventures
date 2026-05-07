# smart-home-core

Repository-owned normalized smart-home model shared by integrations, tools, and
Chief of Staff agents.

This crate is the D23 common vocabulary. Hue, Zigbee, Z-Wave, Thread, Matter,
MQTT, and future adapters project into these same records:

- `Bridge`
- `Device`
- `Entity`
- `Capability`
- `DeviceEvent`
- `DeviceCommand`
- `CommandResult`
- `Scene`
- `StateSnapshot`
- `IntegrationDescriptor`
- `SmartHomeTool` / `ToolDescriptor`
- `CapabilityGrant`

Protocol-private identifiers stay in `ProtocolIdentifier` records rather than
becoming repository-owned entity ids.

## Scope

Current scope:

- normalized bridge/device/entity records
- capability and value typing
- immutable device events and command requests
- command risk tier helpers
- state freshness helpers
- D18D-style smart-home tool descriptors
- agent capability grants for checking tool access before dispatch

Out of scope:

- persistent registry storage
- actor supervision
- HTTP/serial/radio I/O
- Vault leases
- policy execution

## Development

```bash
bash BUILD
```
