# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `ieee802154-core` crate: a dependency-free
  parser/encoder for IEEE 802.15.4 MAC frames (the byte-level foundation for
  Zigbee and Thread).
- Frame control parse/encode; MAC frame parse (with/without FCS) / encode /
  summary / free, including PAN-id compression and the auxiliary security header
  (security control, 32/40-bit frame counter, four key-identifier modes); beacon
  payload parse (superframe spec, GTS descriptors, pending short/extended
  addresses); PAN descriptor derivation from a beacon frame; and PAN-scan
  filtering/ranking helpers. Superframe and security-level accessors included.
- Every multi-byte field is little-endian and bounds-checked, so truncated or
  hostile frames error rather than read out of bounds. Parse-produced payloads
  are heap-owned; bounded MAC counts (GTS/pending ≤ 7) use fixed arrays; the
  encode buffer guards `size_t` overflow. Verified clean under ASan + UBSan, the
  macOS `leaks` tool (0 leaks), and a 300k-iteration random-input fuzz.
- Documented divergences: error enums drop the diagnostic field/needed/remaining
  payloads the Rust carries; `IE_MAC_OK`/`IE_BEACON_OK` added as success
  sentinels.
- 103 checks mirroring the crate's unit tests (frame parse/encode round-trips,
  ack frames, FCS, sequence suppression, reserved-mode rejection, aux security
  header key-index/key-source8, beacon payloads with GTS and pending addresses,
  PAN descriptor + scan ranking, truncation) run under every ISO C compiler via
  the shared `iso-harness`.
