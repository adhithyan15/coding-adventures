# Changelog — image-codec-png (F#)

## 0.1.0 — 2026-08-20

- Added the bounded IC18 encoder and decoder for 8-bit colour types 0, 2, 4,
  and 6.
- Reused F# ZIP for CRC-32, raw RFC 1951 encoding, counted capped decoding,
  and exact-compressed-consumption evidence.
- Added all five filters, suggested `PLTE`, `tRNS`, split IDAT, zlib and Adler
  validation, named APNG refusal, and 29 stable payload-blind failures.
- Added edge, product, exact-inflate, and caller-lowerable allocation limits.
- Added the complete 85-case neutral fixture consumer, independent .NET zlib
  filter inspection, direct error-precedence tests, cross-platform BUILD files,
  90% coverage enforcement, and an empty capability profile.
