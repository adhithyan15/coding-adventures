# Changelog — `aarch64-encoder`

## 0.2.1 — 2026-05-13 (LANG39)

**PC-relative ADRP placeholder for Mach-O data-section relocations.**

### New method

| Method | ARM64 mnemonic | Purpose |
|--------|---------------|---------|
| `adrp_placeholder(rd) → usize` | `ADRP Xd, #0` | Emit a zeroed-immediate ADRP; returns word index for `ARM64_RELOC_PAGE21` relocation |

`adrp_placeholder` is the first half of the ADRP+ADD address-materialisation
pair used by `aarch64-backend` to access `_twig_globals` in the `__data`
section.  The system linker patches the 21-bit `immhi:immlo` field at final
link time.

2 new unit tests verify the encoding and the returned word-index contract.

## 0.2.0 — 2026-05-13 (LANG38)

**Integer division, bitwise logic, variable shifts, unary negate/NOT.**

Added 11 new instruction-emission methods needed by the AOT arithmetic
completeness sprint (LANG38).  All use 64-bit register-register forms.

### New methods

| Method | ARM64 mnemonic | Encoding family |
|--------|---------------|-----------------|
| `sdiv(rd, rn, rm)` | `SDIV Xd, Xn, Xm` | Data-processing 2-source |
| `udiv(rd, rn, rm)` | `UDIV Xd, Xn, Xm` | Data-processing 2-source |
| `msub(rd, rn, rm, ra)` | `MSUB Xd, Xn, Xm, Xa` | Data-processing 3-source |
| `and_(rd, rn, rm)` | `AND Xd, Xn, Xm` | Logical shifted-register |
| `orr(rd, rn, rm)` | `ORR Xd, Xn, Xm` | Logical shifted-register |
| `eor(rd, rn, rm)` | `EOR Xd, Xn, Xm` | Logical shifted-register |
| `mvn(rd, rm)` | `MVN Xd, Xm` | Logical shifted-register (ORN XZR alias) |
| `lsl_reg(rd, rn, rm)` | `LSLV Xd, Xn, Xm` | Data-processing 2-source |
| `lsr_reg(rd, rn, rm)` | `LSRV Xd, Xn, Xm` | Data-processing 2-source |
| `asr_reg(rd, rn, rm)` | `ASRV Xd, Xn, Xm` | Data-processing 2-source |
| `neg_(rd, rm)` | `NEG Xd, Xm` | Arithmetic shifted-register (SUB XZR alias) |

`msub` is the building block for integer modulo: after `sdiv X2, X0, X1`
the remainder is `msub X0, X2, X1, X0` (`X0 = X0 − X2×X1`).

11 new unit tests verify each encoding against known-good bit patterns.

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
