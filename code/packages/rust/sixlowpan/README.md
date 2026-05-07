# sixlowpan

6LoWPAN IPv6 adaptation primitives for Thread over IEEE 802.15.4.

This crate starts D27 below Thread MLE:

- dispatch byte classification
- mesh header hop-limit and short/extended address parsing
- LOWPAN_IPHC first/second byte parsing
- LOWPAN_NHC UDP port compression and checksum-elision parsing
- fragment first/next header parse and encode
- fragment payload parsing and deterministic reassembly buffers
- low-level frame payload extraction

It does not yet perform full IPv6 header decompression, MLE, commissioning, or
border-router behavior.

## Dependencies

- `ieee802154-core`

## Development

```bash
bash BUILD
```
