# Changelog — `aarch64-encoder`

## 0.7.0 — 2026-07-21 (GC stack-map registration — `ADR` + embedded data)

### Added — `adr` / `adr_placeholder` / `emit_data_word`
Primitives the twig-aot GC stack-map registration codegen (`__gc_init_stackmaps`)
needs to point registers at constant tables and at cross-function code addresses:

- **`adr(rd, label)`** — `ADR Xd, <label>`: load the **byte address** of a label
  (PC-relative, ±1 MiB) in one instruction. Unlike `adrp_placeholder` (4 KiB page
  granularity + a companion `ADD`), `ADR` reaches any byte, so it addresses a data
  word embedded in the stream. Resolved at `finish()` via a new `BranchKind::AdrByte`
  fix-up; a target beyond ±1 MiB yields `BranchOutOfRange { bits: 21 }`.
- **`adr_placeholder(rd) -> usize`** — emit `ADR Xd, #0` and return its word index,
  for a caller that patches the 21-bit byte displacement itself once a target offset
  is known (the `ADR` analogue of `adrp_placeholder`). Because `ADR` is PC-relative in
  *bytes*, an intra-section displacement is independent of the runtime base address —
  the key property that lets a code address be baked without a load-time relocation
  (an `ADRP` page immediate cannot, since `page()` does not commute with an unaligned
  base).
- **`emit_data_word(word) -> usize`** — append a raw little-endian `u32` data word
  (a `u32`/`i32` table element), returning its word index. Not an instruction; only
  safe where control flow cannot fall into it (after a `ret`, or reached only by
  `adr`).

Three unit tests pin the `ADR` encoding (`adr x3, .+8` = `0x10000043`), the
zero-displacement case, and the ±1 MiB range error.

## 0.6.0 — 2026-06-28 (AL8-sqrt — `FSQRT Dd, Dn`)

### Added — `fsqrt`

`fsqrt(dd, dn)` emits `FSQRT Dd, Dn` — IEEE-754 double-precision square root.
Encoding: FP data-processing (1 source), `type=01` (double), `opcode=000011`
(FSQRT): `0001_1110_0110_0001_1100_00nn_nnnd_dddd` (`0x1E61C000`).  Single
hardware instruction, no libm call; NaN propagates, negative → NaN.

## 0.5.0 — 2026-06-23 (int ⇄ real conversions — LANG-FULL E8)

### Added — `scvtf` / `fcvtzs` / `frintm`

Three scalar conversion encoders for the E8 numeric-conversion ops:

| Method | Instruction | Encoding | Purpose |
|--------|-------------|----------|---------|
| `scvtf(dd, xn)` | `SCVTF Dd, Xn` | `0x9E620000 \| (Xn<<5) \| Dd` | signed i64 → double (widen, exact ≤2⁵³) |
| `fcvtzs(xd, dn)` | `FCVTZS Xd, Dn` | `0x9E780000 \| (Dn<<5) \| Xd` | double → signed i64, round toward zero |
| `frintm(dd, dn)` | `FRINTM Dd, Dn` | `0x1E654000 \| (Dn<<5) \| Dd` | round double toward −∞ (floor) |

`int_to_real` lowers to `scvtf`; `real_to_int_trunc` to `fcvtzs`;
`real_to_int_floor` to `frintm` then `fcvtzs`. As with `ldr_d`/`str_d`, the
register-file (`Xn` GPR vs `Dn` FP) is selected by the opcode, so callers pass
`Reg::Xk` to name either `Xk` or `Dk`. Unit tests assert the exact bytes for
both base (all-zero) and non-zero register placements.

`fcvtzs` *saturates* on NaN/±∞/out-of-range (ARM never traps) — a documented
divergence from the VM's fail-closed trap, shared with the JVM backend.

## 0.4.0 — 2026-06-20 (scalar double-precision FP — LANG-FULL E3)

### Added — `ldr_d`/`str_d`/`fadd`/`fsub`/`fmul`/`fdiv`/`fcmp` (double)

Seven scalar double-precision floating-point instructions, for ALGOL `real`
(enabler E3) on the native-AOT backend:

- `ldr_d Dt, [Xn, #imm]` / `str_d Dt, [Xn, #imm]` — load/store a 64-bit double
  (`0xFD400000`/`0xFD000000`, scaled-by-8 offset like the `Xt` forms). A
  `float64` value rides its 8-byte stack slot as raw bits.
- `fadd`/`fsub`/`fmul`/`fdiv Dd, Dn, Dm` — double arithmetic
  (`0x1E602800`/`0x1E603800`/`0x1E600800`/`0x1E601800`).
- `fcmp Dn, Dm` — compare two doubles, set NZCV (`0x1E602000`); read with a
  following `cset Xd, <cond>`.

The register number reuses `Reg::idx()` (0–31) — the *opcode* (not the register
field) selects the FP/SIMD register file. **Every encoding was verified
byte-for-byte against the system assembler** (`clang -c` of the same mnemonics)
plus exact-encoding unit tests.

## 0.3.0 — 2026-05-20 (LANG76 — byte memory primitives)

Adds the unsigned-offset byte load/store instructions used by
`aarch64-backend` to lower `load_byte` / `store_byte`:

- `ldrb(rt, rn, imm)` — `LDRB Wt, [Xn, #imm]`, `imm ∈ [0, 4095]`.
  Loads one byte from `[Xn + imm]`, zero-extends to 32 bits in `Wt`
  (which also zeros the upper 32 bits of `Xt` per AArch64 spec).
- `strb(rt, rn, imm)` — `STRB Wt, [Xn, #imm]`, same `imm` range.
  Writes the low byte of `Wt` to `[Xn + imm]`.

Encoding base: `0x39400000` (LDRB) / `0x39000000` (STRB); imm12 in
bits 21..10, Rn in 9..5, Rt in 4..0.  Unlike `LDR Xt` / `STR Xt` the
byte form is **not** scaled — `imm` is interpreted as a raw byte
offset.

## 0.2.2 — 2026-05-13 (LANG40)

**Pre-indexed byte store — `STRB Wt, [Xn, #-1]!`.**

### New method

| Method | ARM64 mnemonic | Encoding |
|--------|---------------|----------|
| `strb_pre_neg1(wt, rn)` | `STRB Wt, [Xn, #-1]!` | `0x381FFC00 \| (Rn << 5) \| Rt` |

`STRB Wt, [Xn, #-1]!` is the ARM64 canonical "push byte" instruction:
it decrements the base register by 1 before storing the low byte of `Wt`.
Used by the `__twig_print_i64` helper (emitted into the text section by
`aarch64-backend`) to write decimal digits backwards into a stack buffer
during the integer-to-ASCII conversion loop.

Note: `wt` uses the same [`Reg`] enum as 64-bit registers; the `size=00`
opcode field makes the hardware treat it as a W (byte) operand.

### Tests

- `strb_pre_neg1_encoding` — canonical `STRB W4, [X5, #-1]!` = `0x381FFCA4`
- `strb_pre_neg1_x0_x0` — degenerate same-register case = `0x381FFC00`

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
