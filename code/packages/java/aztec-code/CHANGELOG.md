# Changelog

## 0.1.0 — 2026-04-27

### Added
- Initial implementation of Aztec Code encoder (ISO/IEC 24778:2008).
- Byte-mode encoding (Binary-Shift from Upper mode).
- Compact (1–4 layer, 15×15–27×27) and Full (1–32 layer, 19×19–143×143) symbols.
- GF(256)/0x12D Reed-Solomon ECC with b=1 convention.
- GF(16) mode message with RS protection.
- Bit stuffing (complement after 4 identical consecutive bits).
- Bullseye finder pattern, orientation marks, reference grid (Full symbols).
- `encode(String)`, `encode(String, AztecOptions)`, `encode(byte[])`, `encode(byte[], AztecOptions)`.
- `AztecOptions` with `minEccPercent` (default 23%).
- `AztecException` and `InputTooLongException` error hierarchy.
