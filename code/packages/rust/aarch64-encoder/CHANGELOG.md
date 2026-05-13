# Changelog — `aarch64-encoder`

## 0.1.1 — 2026-05-13

### Added

- **`ExternalReloc` struct** — records a placeholder `BL #0` instruction site
  for cross-function calls.  Fields: `word_idx: usize` (index into the
  per-function code word array), `symbol: String` (callee name to resolve).
- **`Assembler::bl_external`** — emits `BL #0` (opcode `0x94000000`) at the
  current position, appends an `ExternalReloc`, and returns the word index
  for tracing.
- **`Assembler::external_relocs`** field — collects all `ExternalReloc`
  entries emitted during assembly.  The AOT linker drains this via
  `std::mem::take` after the function body is complete.

## 0.1.0 — 2026-05-05

Initial release.  Pure-Rust ARM64 instruction encoder covering the subset
needed by jit-core / aot-core to lower CIR to native machine code:

- Move-immediate: `movz`, `movk`, plus a `mov_imm64` synthesiser
- Arithmetic (register + 12-bit immediate): `add`, `sub`, `mul`,
  `add_imm`, `sub_imm`
- Compare: `cmp`, `cmp_imm` (aliases for `subs xzr, ...`)
- Memory: `ldr`, `str_` (unsigned-offset, 64-bit)
- Pair: `stp_pre`, `ldp_post` (for prologue / epilogue framing)
- Branches (label-resolved): `b`, `b_cond`, `bl`, `cbz`, `cbnz`
- Indirect: `blr`, `ret`
- Conditional set: `cset`
- System: `svc` (supervisor call)
- Misc: `nop`, `udf`

33 unit tests verify each encoding against known-good bit patterns from
the *ARM Architecture Reference Manual for ARMv8-A* (DDI 0487).
