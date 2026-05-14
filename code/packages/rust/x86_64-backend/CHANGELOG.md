# Changelog — `x86_64-backend`

## 0.3.0 — 2026-05-14 (LANG43 phase 5 — calls + relocations)

Adds cross-function `call` lowering and external relocation surfacing.

**New CIR opcode:**

- `call callee_name, arg0, …, argN` — argument marshalling into the
  ABI's argument registers (System V: RDI/RSI/RDX/RCX/R8/R9 up to 6;
  MS x64: RCX/RDX/R8/R9 up to 4), then `CALL rel32` to the callee, then
  store RAX into the destination slot.
  - Self-recursive calls (callee == current function) emit
    `call_label(entry_label)` resolved within this function's bytes
    by the encoder's fixup pass.
  - Cross-function calls emit `call_rel32(callee_name, PltRel32)` and
    record an external relocation for the AOT linker to patch after
    all function bodies are concatenated.

**New public function:**

- `compile_function_with_relocs(ctx, ir, abi) -> (Vec<u8>, Vec<Reloc>)`
  — returns both the function bytes and the list of external
  relocations.  `compile_function` is unchanged for callers that
  don't need them.

Re-exports `x86_64_encoder::ExternalReloc as Reloc` for parity with
`aarch64-backend`'s `Reloc` re-export.

## 0.2.0 — 2026-05-14 (LANG43 — LANG38-parity wave)

Extends the V1 backend with the same opcodes `aarch64-backend` gained
in its LANG38 release.  Same CIR coverage now compiles on both
backends.

**New CIR opcodes:**

- `div_<ty>`, `mod_<ty>` — integer division and modulo.  Signed types
  use `CQO` + `IDIV`; unsigned types use `XOR rdx, rdx` + `DIV`.
  Quotient lives in `RAX`, remainder in `RDX` (sequenced by hand —
  no register-allocator surprises because RAX/RCX/RDX were already
  reserved in V1).
- `and_<ty>`, `or_<ty>`, `xor_<ty>` — bitwise logical (64-bit).
- `not_<ty>` — bitwise complement (`NOT r/m64`).
- `shl_<ty>` — logical shift left (`SHL r/m64, CL`).
- `shr_<ty>` — arithmetic shift right (`SAR`) for signed types,
  logical shift right (`SHR`) for unsigned types.
- `neg_<ty>` — two's-complement negate (`NEG r/m64`).

All shifts use `CL` as the count register (x86-64 ISA constraint);
the backend pre-loads `rhs` into `RCX` before issuing the shift.

Still out of scope (added by later phases):
- Calls + external relocations (phase 5)
- Globals + `io_out` (phase 6)
- Floats / closures

## 0.1.0 — 2026-05-14 (LANG43)

Initial release.  V1 backend matching the `aarch64-backend` V1 baseline.

**ABIs supported:**

- System V AMD64 (Linux, macOS x86-64, FreeBSD) — arg regs RDI/RSI/RDX/RCX/R8/R9
- Microsoft x64 (Windows) — arg regs RCX/RDX/R8/R9, 32-byte shadow space reserved in prologue

Both ABIs share the same CIR lowering logic — only the prologue's arg
register set and shadow-space allocation differ.

**CIR coverage:**

- `const_<ty>` — integer + bool literals
- `mov_<ty>` — typed copy
- `add_<ty>`, `sub_<ty>`, `mul_<ty>` — integer arithmetic (64-bit; result not masked to width)
- `cmp_<rel>_<ty>` — signed and unsigned comparisons, 6 predicates × signed/unsigned
- `label`, `jmp`, `jmp_if_true`, `jmp_if_false` — control flow
- `ret_<ty>`, `ret_void` — return (loads value into RAX before epilogue)
- `type_assert` — lowered to `UD2` trap (AOT has no deopt path)

**Out of scope for V1 (added by follow-up phases):**

- Division / modulo (`IDIV` / `DIV` — phase 4)
- Logical (AND/OR/XOR/NOT) and shifts (phase 4)
- Calls + external relocations (phase 5)
- Globals + `io_out` (phase 6)
- Floats / SSE
- Closures
- Local register allocator (V1 uses pure stack spill)

**Register allocation:** stack spill.  Every virtual lives at
`[rbp - 8 - slot_idx*8]`.  RAX, RCX, RDX reserved as scratch.
