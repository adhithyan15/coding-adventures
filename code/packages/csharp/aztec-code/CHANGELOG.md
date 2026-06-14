# Changelog - CodingAdventures.AztecCode.CSharp

## [0.1.0] - 2026-04-27

### Added

- Initial native C# Aztec Code encoder.
- Byte-mode compact/full symbol selection with tunable minimum ECC.
- GF(256)/0x12D Reed-Solomon data ECC and GF(16) mode-message ECC.
- Bullseye, orientation marks, full-symbol reference grid, bit stuffing, and
  clockwise data placement.
- Unit tests covering symbol sizing, geometry, determinism, byte payloads,
  UTF-8 input, and option validation.
