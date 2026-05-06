# thread-mle

Thread Mesh Link Establishment roles, TLVs, and attach-state primitives.

This crate starts the D27 Thread control-plane layer above 6LoWPAN:

- device role model
- MLE command ids
- common MLE TLV ids
- MLE message/TLV parsing and encoding
- scan-mask and mode bit helpers
- deterministic parent/child attach-state skeleton

It does not yet implement UDP, CoAP, DTLS, commissioning, network data,
neighbor tables, border routing, or real radio behavior.

## Dependencies

- `sixlowpan`

## Development

```bash
bash BUILD
```
