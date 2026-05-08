# smart-home-discovery

Pure discovery-record primitives for the D23 smart-home runtime.

This crate does not open sockets, send mDNS packets, call vendor cloud APIs, or
write credentials. It gives discovery workers a shared shape for:

- mDNS/SSDP/cloud/manual discovery sources
- bridge candidate records with stable integration/native identifiers
- manual bridge address normalization
- mDNS advertisement endpoint helpers
- deterministic candidate catalogs
- source/address/time preference scoring for duplicate bridge candidates
- freshness filtering for supervisor/discovery loops
- projection into unpaired `smart-home-core::Bridge` records

Network transports, Hue-specific discovery, Vault credential storage, and actor
supervision live in later integration/runtime crates.

## Dependencies

- smart-home-core

## Development

```bash
bash BUILD
```
