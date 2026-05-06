# zigbee-zdo

Zigbee Device Object descriptor and discovery primitives.

This crate starts the D25 device-interview layer above APS. It provides:

- ZDO cluster ids for descriptor, endpoint, bind, and management requests
- node descriptor parsing
- simple descriptor parsing
- active endpoint response parsing
- APS request builders for node/simple descriptor and active endpoint requests
- interview-summary projection into a normalized D23 `Device` skeleton

## Dependencies

- smart-home-core
- zigbee-nwk
- zigbee-aps

## Development

```bash
# Run tests
bash BUILD
```
