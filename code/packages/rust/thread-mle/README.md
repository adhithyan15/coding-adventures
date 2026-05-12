# thread-mle

Thread Mesh Link Establishment roles, TLVs, and attach-state primitives.

This crate starts the D27 Thread control-plane layer above 6LoWPAN:

- device role model
- MLE command ids
- common MLE TLV ids
- MLE message/TLV parsing and encoding
- scan-mask and mode bit helpers
- typed Leader Data TLV helpers and opaque Network Data extraction
- Thread Network Data TLV parsing/encoding with stable-bit preservation
- typed Prefix TLV projection with nested Network Data sub-TLVs
- compact Network Data summaries for top-level TLV, prefix, stability, routing,
  service, and unknown-TLV coverage
- typed Connectivity TLV helpers for route-cost and active-router diagnostics
- Thread diagnostic snapshots that combine neighbor health with leader,
  connectivity, partition, and prefix data from MLE messages
- supervisor action projections that turn diagnostic drift into stable repair
  intents for runtime supervisors
- neighbor table summaries for cheap runtime/read-model projections of parent,
  child, router, stale-neighbor, and parent-candidate state
- deterministic parent/child attach-state skeleton
- neighbor table primitives for parent/child/router relationships, link margin,
  timeout freshness, and parent-candidate selection

It does not yet implement UDP, CoAP, DTLS, commissioning, border-router routing
policy, or real radio behavior.

## Dependencies

- `sixlowpan`

## Development

```bash
bash BUILD
```
