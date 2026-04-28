# Changelog

All notable changes to `CodingAdventures::AztecCode` are documented here.

## [0.1.0] — 2026-04-26

### Added

- Initial release: ISO/IEC 24778:2008 compliant Aztec Code encoder.
- Compact symbols (1–4 layers, sizes 15×15, 19×19, 23×23, 27×27).
- Full symbols (1–32 layers, sizes 19×19 up to 143×143).
- Auto-select between compact and full at the chosen ECC level.
- Byte-mode (Binary-Shift from Upper) data encoding for any input bytes.
- Reed-Solomon error correction over GF(256)/0x12D (Data Matrix polynomial,
  generator roots α¹..αⁿ) for 8-bit data codewords.
- Reed-Solomon over GF(16) (primitive polynomial 0x13) for the mode message.
- Bit stuffing: insert a complement bit after every run of 4 identical bits.
- Bullseye finder pattern with concentric dark/light rings (radius 5 compact,
  radius 7 full).
- Reference grid every 16 modules for full symbols.
- Orientation marks at the 4 corners of the mode-message ring.
- Clockwise spiral data placement, 2-modules-wide bands per layer.
- Public API: `encode($data, \%options)`. Options: `min_ecc_percent`
  (default 23, range 10–90).
- Returns a `ModuleGrid` hashref compatible with
  `CodingAdventures::Barcode2D` (`rows`, `cols`, `modules` AoA, `module_shape`).
- Croaks with `InputTooLong: ...` when data exceeds 32-layer full capacity.
- Test suite with 22 subtests covering compact and full sizes, bullseye and
  orientation invariants, byte-array equivalence, ECC option, determinism,
  unicode/binary corpora, and the input-too-long error path.
