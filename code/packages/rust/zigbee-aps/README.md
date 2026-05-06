# zigbee-aps

Zigbee Application Support Sublayer frame primitives for endpoints, clusters,
groups, and counters.

This crate sits above `zigbee-nwk` and below ZDO/ZCL. It owns the APS byte
boundary:

- APS frame control parsing and encoding
- delivery modes: unicast, indirect, broadcast, group
- endpoint and group addressing
- cluster/profile ids
- APS counters
- payload preservation

It does not yet implement binding tables, APS commands, fragmentation, security,
ZDO discovery, or ZCL command semantics.

## Dependencies

- `zigbee-nwk`

## Development

```bash
bash BUILD
```
