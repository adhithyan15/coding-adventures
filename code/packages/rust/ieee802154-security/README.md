# ieee802154-security

From-scratch IEEE 802.15.4 security primitives.

This package sits immediately above `ieee802154-core`. The core package parses
frame structure. This package starts turning that structure into the inputs a
security engine needs: CCM* nonces, replay checks, and key lookup.

## Current Scope

- CCM* nonce construction for the IEEE 802.15.4-2006 style nonce:
  `source extended address || frame counter || security level`
- security source address extraction
- replay-window acceptance for monotonic incoming frame counters
- key identifier normalization
- small in-memory key store for tests and simulators

## Not Yet Implemented

- AES-CCM* encryption/decryption
- MIC verification
- associated-data construction
- outgoing frame counter allocation
- persistent replay databases
- integration with Vault-backed real key custody

## Layer Position

```text
zigbee-security / thread-security
    |
    v
ieee802154-security
    |
    v
ieee802154-core
```

This package is still pure computation. It does not know about radios, files,
Vault, or OS services.
