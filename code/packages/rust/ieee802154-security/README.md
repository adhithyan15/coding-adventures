# ieee802154-security

From-scratch IEEE 802.15.4 security primitives.

This package sits immediately above `ieee802154-core`. The core package parses
frame structure. This package starts turning that structure into the inputs a
security engine needs: CCM* nonces, secured-frame byte accounting, AES-CCM*
encryption/decryption, replay checks, and key lookup.

## Current Scope

- CCM* nonce construction for the IEEE 802.15.4-2006 style nonce:
  `source extended address || frame counter || security level`
- secured-frame byte material extraction:
  authenticated header bytes, encrypted payload bytes, and MIC bytes
- AES-CCM* encryption/decryption for 13-byte nonces and the 32-bit frame-counter
  profile used by the first 802.15.4 security track
- security source address extraction
- replay-window acceptance for monotonic incoming frame counters
- key identifier normalization
- small in-memory key store for tests and simulators

## Not Yet Implemented

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

## Secured Frame Parts

`SecuredFrameParts::from_frame` turns a parsed `MacFrame` into the byte slices
that the later AES-CCM* layer will need:

```text
MacFrame
  -> SecurityContext
  -> CCM* nonce
  -> authenticated_data    frame header + addressing + auxiliary security header
  -> encrypted_payload     payload bytes excluding trailing MIC
  -> mic                   trailing bytes selected by security level
```

The package does not decrypt, encrypt, or authenticate yet. It only performs
the byte accounting so that the AES-CCM* layer can stay small and testable.

## AES-CCM*

`ccm_star_encrypt` and `ccm_star_decrypt` implement the first AES-CCM* profile
used by this stack:

- AES-128 keys
- 13-byte nonces
- 2-byte CCM length/counter field
- MIC lengths selected from the IEEE 802.15.4 security level
- encryption controlled by the security level
- constant-time MIC comparison during decrypt

The implementation is checked against RFC 3610 packet vector #1 for the
underlying CCM byte layout. Higher-level Zigbee and Thread packages will decide
which keys, frame counters, and replay state apply to a given frame.
