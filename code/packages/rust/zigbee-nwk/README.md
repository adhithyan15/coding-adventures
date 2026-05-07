# zigbee-nwk

Zigbee network-layer frame and address primitives built above IEEE 802.15.4.

This crate starts D25 at the NWK byte boundary:

- 16-bit network addresses
- extended IEEE addresses
- NWK frame-control fields
- radius and sequence fields
- optional extended source/destination addresses
- optional multicast control byte
- neighbor table primitives for router/end-device relationships and freshness
- route table primitives for destination-to-next-hop lookups
- payload extraction and round-trip encoding

It intentionally does not implement APS, ZDO, ZCL, route discovery protocol
messages, joining, security policy, or coordinator behavior yet.

## Dependencies

- `ieee802154-core`

## Development

```bash
bash BUILD
```
