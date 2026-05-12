# zigbee-zcl

Zigbee Cluster Library frame, attribute, and D23 mapping primitives.

This crate starts the D25 cluster-library layer without radio, APS transport, or
coordinator policy. It provides:

- ZCL cluster and attribute identifiers for common smart-home clusters
- foundation and cluster-specific frame control parsing/encoding
- read-attributes, on/off, level, and color-temperature command frame builders
- typed attribute report parsing
- compact attribute report summaries for parsed report shape and D23 delta
  coverage
- D23 capability projection for common clusters
- D23 `StateDelta` projection for on/off, level, color-temperature, occupancy,
  lock-state, temperature, humidity, and illuminance reports
- endpoint references grounded in `zigbee-nwk` network addresses

## Dependencies

- smart-home-core
- zigbee-nwk

## Development

```bash
# Run tests
bash BUILD
```
