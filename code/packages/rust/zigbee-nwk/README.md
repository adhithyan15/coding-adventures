# zigbee-nwk

Zigbee network-layer frame and address primitives built above IEEE 802.15.4.

This crate starts D25 at the NWK byte boundary:

- 16-bit network addresses
- extended IEEE addresses
- NWK frame-control fields
- radius and sequence fields
- optional extended source/destination addresses
- optional multicast control byte
- optional source-route relay subframes
- neighbor table primitives for router/end-device relationships and freshness
- route table primitives for destination-to-next-hop lookups
- topology summaries for neighbor roles, relationships, identity coverage,
  link metric extrema, freshness, depth, router candidates, and route health
  supervision
- routing readiness summaries that project neighbor freshness, active route
  coverage, route-discovery needs, and supervision flags for mesh forwarding
- route-discovery command summaries for request/reply/status/record traffic,
  IEEE-address coverage, multicast and many-to-one flags, route records, and
  repair-needed status signals
- route-repair readiness summaries that combine routing readiness with
  route-discovery command telemetry and blocker checks
- typed NWK route-discovery command payloads:
  - route request
  - route reply
  - network status
  - route record
- payload extraction and round-trip encoding

It intentionally does not implement APS, ZDO, ZCL, joining, security policy, or
coordinator behavior yet.

## Dependencies

- `ieee802154-core`

## Development

```bash
bash BUILD
```
