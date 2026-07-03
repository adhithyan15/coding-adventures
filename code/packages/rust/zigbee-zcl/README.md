# zigbee-zcl

Zigbee Cluster Library frame, attribute, and D23 mapping primitives.

This crate starts the D25 cluster-library layer without radio, APS transport, or
coordinator policy. It provides:

- ZCL cluster and attribute identifiers for common smart-home clusters
- foundation and cluster-specific frame control parsing/encoding
- payload-free ZCL frame summaries for routing/default-response diagnostics
- ZCL frame batch summaries for payload-free parser and bridge telemetry
  rollups
- read-attributes, on/off, level, and color-temperature command frame builders
- Default Response foundation frame builders with typed ZCL status codes
- typed attribute report parsing
- typed attribute report encoding and Report Attributes frame builders
- compact attribute report summaries for parsed report shape and D23 delta
  coverage
- attribute report readiness summaries for D23 delta coverage, typed value
  coverage, and raw/unknown-type hygiene checks
- ZCL report operator summaries that combine frame telemetry with attribute
  report readiness for capture/backlog/review routing
- ZCL report signoff summaries that turn operator readiness into final
  report capture signoff checks
- ZCL report closure summaries that turn signoff readiness into final
  report closure checks
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
