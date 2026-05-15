# Changelog — `x86_64-backend`

## 0.4.0 — 2026-05-14 (LANG43 phase 6 — globals + io_out)

Adds the last CIR opcodes needed to match `aarch64-backend`'s
LANG39/LANG40/LANG41 coverage.  After this release, the same Twig
programs that compile and run end-to-end on macOS ARM64 will compile
and run end-to-end on Linux x86-64 and Windows x86-64 once LANG45
(object emitters) and LANG46 (twig-aot driver) land.

**New CIR opcodes:**

- `global_load name → dest` — read from a slot in the `_twig_globals`
  data section:
  ```
  lea  rax, [rip + _twig_globals]   ; PcRel32 reloc
  mov  rax, [rax + slot*8]          ; load 64-bit value
  mov  [rbp + dest_slot], rax
  ```
  Note that x86-64's RIP-relative addressing collapses ARM64's
  ADRP+ADD pair into a single `LEA` + a single `PcRel32` reloc
  record — much simpler than the AArch64 `GlobalWordReloc` shape.

- `global_store name, val` — write to a slot:
  ```
  mov  rcx, [val_slot]               ; load value
  lea  rax, [rip + _twig_globals]    ; PcRel32 reloc
  mov  [rax + slot*8], rcx
  ```

- `io_out val` — call `__twig_print_i64`:
  ```
  mov  <arg0>, [rbp + val_slot]      ; SysV: RDI; MS x64: RCX
  call __twig_print_i64               ; PltRel32 reloc
  ```
  Stack alignment at the call is correct without per-call adjustment:
  the prologue established RSP ≡ 0 (mod 16), and CALL pushes 8 bytes
  for the return address, giving RSP ≡ 8 (mod 16) at helper entry —
  exactly what the ABI requires.  MS x64 shadow space is already
  reserved in the prologue.

**New public function:**

- `compile_function_with_globals(ctx, ir, abi, global_slots) ->
  (Vec<u8>, Vec<Reloc>)` — resolves global names through `global_slots`
  into slot indices and emits the corresponding `PcRel32` relocs
  alongside any cross-function / runtime `PltRel32` relocs.

`compile_function` and `compile_function_with_relocs` are unchanged
for callers that don't use globals (passing an empty slot map is
equivalent).

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
