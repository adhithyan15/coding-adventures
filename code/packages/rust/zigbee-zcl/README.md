# zigbee-zcl

Zigbee Cluster Library frame, attribute, and D23 mapping primitives.

This crate starts the D25 cluster-library layer without radio, APS transport, or
coordinator policy. It provides:

- ZCL cluster and attribute identifiers for common smart-home clusters
- foundation and cluster-specific frame control parsing/encoding
- read-attributes and on/off command frame builders
- typed attribute report parsing
- D23 capability projection for common clusters
- D23 `StateDelta` projection for on/off, level, color-temperature, occupancy,
  and lock-state reports
- endpoint references grounded in `zigbee-nwk` network addresses

## Dependencies

- smart-home-core
- zigbee-nwk

## Development

```bash
# Run tests
bash BUILD
```
