# zigbee-zdo

Zigbee Device Object descriptor and discovery primitives.

This crate starts the D25 device-interview layer above APS. It provides:

- ZDO cluster ids for descriptor, endpoint, bind, and management requests
- node descriptor parsing
- simple descriptor parsing
- active endpoint response parsing
- APS request builders for node/simple descriptor and active endpoint requests
- APS request builders and status parsers for bind/unbind requests
- deterministic interview planning for the next ZDO descriptor request
- compact interview-plan summaries for pending descriptor work
- interview-summary projection into a normalized D23 `Device` skeleton
- compact node/simple descriptor and interview read summaries for discovery
  tools that need endpoint and cluster coverage without carrying full payloads

## Dependencies

- smart-home-core
- zigbee-nwk
- zigbee-aps

## Development

```bash
# Run tests
bash BUILD
```
