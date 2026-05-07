# thread-mle

Thread Mesh Link Establishment roles, TLVs, and attach-state primitives.

This crate starts the D27 Thread control-plane layer above 6LoWPAN:

- device role model
- MLE command ids
- common MLE TLV ids
- MLE message/TLV parsing and encoding
- scan-mask and mode bit helpers
- typed Leader Data TLV helpers and opaque Network Data extraction
- deterministic parent/child attach-state skeleton
- neighbor table primitives for parent/child/router relationships, link margin,
  timeout freshness, and parent-candidate selection

It does not yet implement UDP, CoAP, DTLS, commissioning, network data,
border routing, or real radio behavior.

## Dependencies

- `sixlowpan`

## Development

```bash
bash BUILD
```
