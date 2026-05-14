# Changelog — `x86_64-encoder`

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
