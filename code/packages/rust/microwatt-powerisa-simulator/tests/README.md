# Microwatt oracle evidence

The implementation and its source-locked boundary tests use Microwatt commit
`f341bed306a9a7281bf65200df8cc1f0a002617d` and Power ISA Version 3.0C PDF
SHA-256
`0c0064eda27161dcddf52f69a5d26799aaadc5ec73a2de9d90ed09a6ce015010`.

`source_locked.rs` records the reviewed RTL choices whose generic Power ISA
result is not enough to define this profile:

- `execute1.vhdl` sets sticky SO only from 64-bit OV, not OV32;
- `execute1.vhdl` duplicates the `mulhw[u]` high word into both halves;
- `divider.vhdl` zeroes the upper half of `divw[u]`; and
- Rc observes the complete shaped 64-bit destination.

GHDL was not present on the completion host (`command -v ghdl` returned no
path), so this cell does **not** claim that an RTL differential ran. Installing
a system-wide compiler was outside this package-scoped change. A future RTL
run must use GHDL's VHDL-2008 mode, the exact pinned commit above, FPU disabled,
and fixed 64-bit little-endian mode; it must seed and export all 32 GPRs, CR,
XER, LR, CTR, CIA, and bounded memory. Its checked-in manifest must record the
GHDL version, complete harness command, seed, row count, and vector-content
SHA-256. Hand-derived rows in this package must never be relabeled as a GHDL
pass.
