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
- `AuthorizationDecision`

Protocol-private identifiers stay in `ProtocolIdentifier` records rather than
becoming repository-owned entity ids.

## Scope

Current scope:

- normalized bridge/device/entity records
- capability and value typing
- capability-surface summaries for describe-capabilities tools
- inventory summaries for bridge/device health and entity state coverage
- canonical capability catalog entries for light, scene, lock, climate, sensor,
  and input families
- canonical integration descriptors for Hue, Zigbee, Z-Wave, Thread, Matter,
  and MQTT bootstrap families
- compact integration catalog summaries for runtime, discovery, pairing, and
  capability coverage
- immutable device events and command requests
- health and command-result status helpers for supervision/read-side loops
- command risk tier helpers
- state freshness helpers
- D18D-style smart-home tool descriptors
- compact smart-home tool catalog summaries for read-side inspection
- read-only supervision observation tool descriptor for Chief of Staff status
  loops
- agent capability grants for checking tool access before dispatch
- authorization decisions that can be logged by runtimes and agents
- MQTT topic names, topic filters, QoS levels, topic roles, and topic bindings
  for MQTT-backed integrations

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
