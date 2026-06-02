# Changelog — iir-to-ge225

All notable changes to this crate are documented here.

## v0.1.0 — 2026-06-02 — A5 skeleton

Initial release.  Establishes the IIR → GE-225 backend's public
surface as the fifth architecture-backend slot (after iir-to-riscv,
iir-to-intel8008, iir-to-armv7, iir-to-intel4004) — and the most
exotic by a wide margin (20-bit words, mainframe accumulator model,
1959 silicon).

### Added

- `IIRGe225Config` — module-name-carrying config (reserved for
  future symbol-table / `.bin` header use).
- `IIRGe225Error` — backend-side error type with four variants:
  `ValidationFailed`, `UnsupportedOp`, `UnsupportedType`,
  `InvalidOperand`.  Mirrors the iir-to-intel4004 / iir-to-armv7 /
  iir-to-intel8008 / iir-to-riscv error surface so callers can
  pattern-match identically across backends.
- `validate_for_ge225(&IIRModule) -> Vec<String>` — stub validator
  (always returns `[]` in v0.1.0).
- `lower_iir_to_ge225(&IIRModule, &IIRGe225Config) -> Result<Vec<u8>,
  IIRGe225Error>` — lowering entry point.  Currently emits the
  3-byte canonical HLT sentinel regardless of input.
- `pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00]` — the all-zeros
  20-bit GE-225 HLT word, packed big-endian.  Documented choice
  (vs branch-to-self / unimplemented-opcode) recorded in the spec
  and the constant's doc comment.

### Why the all-zeros HLT halt sentinel?

The GE-225's `HLT` instruction is the all-zeros 20-bit word.
Emitted at the start of program ROM, it halts the machine
deterministically — recognized by every GE-225 simulator and the
historical silicon.  Alternative halt idioms (branch-to-self) would
work but produce less visually obvious bytes.

### Word packing

GE-225 words are 20 bits; we pack each into 3 bytes (24 bits),
big-endian, with the top 4 bits of byte 0 always zero.  A downstream
simulator reads 3 bytes per instruction, masks off the top 4 bits,
and recovers the original 20-bit word.

### Scope notes

- No instruction lowering — deferred to v0.2.0 (A5+).
- No `lang-aot --emit=ge225` wiring — deferred to A5+++.
- No external assembler / linker integration.

### Tests

7 tests covering: empty-module validation, output shape, exact
halt bytes, `HALT_WORD` constant pinning, default-config invariant,
`IIRGe225Config::new` builder contract, and Display smoke for all
four `IIRGe225Error` variants.

### Reference

- Spec: `code/specs/iir-to-ge225.md`
- Plan: `code/specs/MULTILANG-ARCHITECTURE-BACKENDS.md` §A5
