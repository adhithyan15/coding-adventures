# Changelog — `x86_64-encoder`

## 0.7.0 — 2026-07-23 — RIP-relative `lea` primitives (GC stack-map registration)

Three additive primitives the twig-aot x86-64 GC stack-map registration codegen
(`__gc_init_stackmaps`, AOT00-T1 x86_64 PR-x3) needs to point registers at constant
tables and cross-function code addresses — the x86-64 analogue of the aarch64 encoder's
`adr` / `adr_placeholder` / `emit_data_word`:

- **`lea_rip_label(dst, label)`** — `LEA r64, [RIP + <label>]` to a label bound later in
  the same function, resolved at `finish()` via the existing fix-up mechanism (like a
  `jmp`). No relocation — the target is intra-function. Points a register at a data word
  embedded in the stream.
- **`lea_rip_placeholder(dst) -> usize`** — `LEA r64, [RIP + #0]` returning its `disp32`
  slot offset, for a caller that patches the displacement once a cross-function target
  offset is known (`disp32 = target − (slot + 4)`). Base-independent (RIP cancels the
  load base), so no relocation is needed — the x86-64 counterpart of the aarch64 `ADR`
  used for `func_start`.
- **`emit_data_u32(word) -> usize`** — append a raw little-endian `u32` table element
  (constant data, not an instruction), returning its byte offset.

Three unit tests (byte-exact `48 8D …` encodings + a data-pool round trip + iced-x86
decode confirming the `lea`). Purely additive; existing encodings unchanged.

### Added — `sqrtsd`

`sqrtsd(xmm_dst, xmm_src)` emits `SQRTSD xmm_dst, xmm_src` — SSE2 double-
precision hardware square root.  Opcode: `F2 0F 51 /r` — e.g. `sqrtsd xmm0,
xmm0` = `F2 0F 51 C0`.  Single instruction, no libm call; NaN propagates,
negative → NaN per IEEE-754.

## 0.5.0 — 2026-06-23 (int ⇄ real conversions — LANG-FULL E8 PR-6b)

### Added — `cvtsi2sd` / `cvttsd2si` / `roundsd`

Three SSE conversion encoders for the E8 numeric-conversion ops, plus two new
private emit helpers they need:

| Method | Instruction | Encoding | Purpose |
|--------|-------------|----------|---------|
| `cvtsi2sd(xmm_dst, gpr_src)` | `CVTSI2SD xmm, r/m64` | `F2 REX.W 0F 2A /r` | signed i64 → double |
| `cvttsd2si(gpr_dst, xmm_src)` | `CVTTSD2SI r64, xmm/m64` | `F2 REX.W 0F 2C /r` | double → i64, toward zero |
| `roundsd(xmm_dst, xmm_src, imm8)` | `ROUNDSD xmm, xmm, ib` | `66 0F 3A 0B /r ib` | round under `imm8` (1 = floor) |

- `emit_sse_rr_w` — SSE reg/reg with a **mandatory REX.W** (the existing
  `emit_sse_rr` only adds a REX byte for high registers and never sets W;
  `cvtsi2sd`/`cvttsd2si` mix a 64-bit GPR with an XMM and require it).
- `emit_sse_rri_0f3a` — the **three-byte** `66 0F 3A <op>` form with a trailing
  `imm8` (used by `roundsd`).

Exact bytes unit-tested for the base (xmm0/rax) encodings and for ModRM.reg /
ModRM.rm placement with REX.R/REX.B extension (e.g. `cvtsi2sd xmm1,r8` =
`F2 49 0F 2A C8`). `cvttsd2si` yields the integer-indefinite `0x8000…0` on
NaN/±∞/out-of-range (no trap) — a documented divergence from the VM trap, shared
with the JVM/aarch64 backends.

## 0.4.0 — 2026-06-20 (SSE2 scalar double-precision FP — LANG-FULL E3)

### Added — `movsd`/`addsd`/`subsd`/`mulsd`/`divsd`/`ucomisd` (double)

Scalar double-precision SSE2 instructions, for ALGOL `real` (enabler E3) on the
native-AOT backend:

- `movsd_load`/`movsd_store` — load/store a 64-bit double (`F2 0F 10`/`F2 0F 11`,
  `[base + disp32]`). A `float64` value rides its 8-byte stack slot as raw bits.
- `addsd`/`subsd`/`mulsd`/`divsd xmm_dst, xmm_src` — double arithmetic
  (`F2 0F 58`/`5C`/`59`/`5E`).
- `ucomisd xmm_a, xmm_b` — unordered double compare, sets `ZF`/`PF`/`CF`
  (`66 0F 2E`); read with `setcc` (NaN sets `PF`).

The `Reg` numbers double as XMM numbers (`Rax`→`xmm0`, …) — the mandatory prefix
+ `0F` opcode select the XMM register file; REX is emitted only for high
registers, always `W=0`. **The reg-reg opcodes are byte-identical to the system
assembler** (`clang -masm=intel`); the mem forms use the encoder's existing
`disp32` policy. Exact-encoding unit tests included.

## 0.3.0 — 2026-05-20 (LANG76 — byte memory primitives)

Two new instruction emitters for `load_byte` / `store_byte` lowering
in `x86_64-backend`:

- `movzx_r64_byte_at(dst, base)` — `MOVZX r64, BYTE PTR [base]` (4
  bytes: `REX.W 0F B6 ModRM`).  Loads one byte from `[base]`,
  zero-extends to 64 bits, writes into `dst`.
- `mov_byte_at_r8(base, src)` — `MOV BYTE PTR [base], src.low8` (3
  bytes: `REX 88 ModRM`).  Always emits a REX prefix (even when
  empty) so the byte-register encoding is unambiguous for any GPR.

Both helpers assert `base.low3() ∉ {4, 5}` (no RSP/R12 SIB; no
RBP/R13 RIP-relative).  Callers always pre-compute the effective
address into RAX/RCX/RDX before invoking these helpers, matching the
LANG76 spec.

## 0.2.0 — 2026-05-14 (LANG43 phase 5 — calls)

Added `call_label(LabelId)` — `CALL rel32` to an internal label,
resolved at `finish()` time exactly like `jmp` / `jcc`.  Used by
`x86_64-backend` for self-recursive calls where the callee lives at
a label inside the same function's bytes; cross-function calls
continue to use `call_rel32(symbol, kind)` with an external
relocation.

## 0.1.0 — 2026-05-14 (LANG44)

Initial release.  Covers the V1 instruction set from LANG44:

- **GPR encoding** — all 16 GPRs (RAX–R15) via REX.W prefix.
- **Move** — `mov_r64_r64`, `mov_r64_imm32` (sign-extended),
  `mov_r64_imm64` (`MOVABS`), `mov_r64_mem`, `mov_mem_r64`.
- **Arithmetic** — `add`, `sub`, `imul`, `add_imm32`, `sub_imm32`,
  `neg_`, `idiv`, `div`, `cqo`.
- **Logical** — `and_`, `or_`, `xor_`, `not_`, `test_`.
- **Shifts** — variable (`shl_cl`, `shr_cl`, `sar_cl`) and immediate
  (`shl_imm8`, `shr_imm8`, `sar_imm8`).
- **Compare + set** — `cmp`, `cmp_imm32`, `setcc`, `movzx_r64_r8`.
- **Memory addressing** — `[base + disp32]`, `[RIP + disp32]` via
  `lea_rip_rel` (with external relocation).
- **Control flow** — `jcc(cond, label)`, `jmp(label)` — always emitted
  in Rel32 form so byte length is fixed at emission time.
- **Calls** — `call_rel32` (with external relocation), `call_r64`,
  `ret`.
- **Stack** — `push`, `pop` (64-bit).
- **Misc** — `nop`, `int3`, `ud2` (deopt trap).

External relocations surfaced as abstract `ExternalRelocKind`
(`PltRel32`, `PcRel32`, `GotPcRel32`); LANG45's `code-packager`
translates to the OS-specific reloc type ID (ELF `R_X86_64_*`, PE/COFF
`IMAGE_REL_AMD64_*`) at emit time.

Zero runtime dependencies.  `iced-x86` is a `#[cfg(test)]` dev-dep
used only for round-trip decode verification.

**Out of scope for V1** (deferred):
- Floats / SSE / AVX
- 16/32-bit operand-size prefixes
- Atomic ops (`LOCK` prefix)
- Short-displacement (Rel8) branch relaxation
