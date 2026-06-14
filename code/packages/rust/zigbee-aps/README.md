# zigbee-aps

Zigbee Application Support Sublayer frame primitives for endpoints, clusters,
groups, and counters.

This crate sits above `zigbee-nwk` and below ZDO/ZCL. It owns the APS byte
boundary:

- APS frame control parsing and encoding
- delivery modes: unicast, indirect, broadcast, group
- endpoint and group addressing
- cluster/profile ids
- cluster/profile classification helpers
- binding table records for device and group destinations
- body-free frame, frame-batch, and binding-table summaries for read-side
  supervision
- frame-batch summary counts for delivery modes, profile/cluster families,
  security, ack requests, and payload volume
- frame-batch readiness summaries for application-delivery, home-automation,
  cluster, and payload capture checks
- delivery handoff summaries that combine frame-batch readiness with
  application-delivery, payload, and security/ack context checks
- binding summary counts for source endpoint shape and cluster coverage
- binding readiness summaries for application-source, destination, cluster, and
  source-endpoint hygiene checks
- APS command identifiers and payload preservation for key-management commands
- APS counters
- payload preservation

It does not yet implement typed APS command bodies, fragmentation, security, ZDO
discovery, or ZCL command semantics.

## Dependencies

- `zigbee-nwk`

## Development

```bash
bash BUILD
```
