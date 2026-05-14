# Changelog — `x86_64-backend`

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
