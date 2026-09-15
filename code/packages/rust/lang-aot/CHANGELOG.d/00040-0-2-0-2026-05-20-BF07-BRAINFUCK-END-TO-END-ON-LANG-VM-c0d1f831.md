## 0.2.0 — 2026-05-20 (BF07 — Brainfuck end-to-end on LANG VM)

Brainfuck programs now compile all the way to a native executable via
`lang-aot foo.bf`.

**New BF lowering pass.**  `lower_brainfuck_for_aot(&mut IIRModule)`
runs after `brainfuck_iir_compiler::compile_source` returns and
rewrites the BF-shaped IIR into a LANG76-shaped one without modifying
the frontend (so existing consumers — `vm-core`, `jit-core`,
`iir-to-wasm` — keep working unchanged):

- Prepends `const __bf_tape_size = 30000` + `alloc_bytes
  __bf_tape_size -> __bf_tape` to `main`.
- Rewrites `load_mem v, ptr` → `load_byte __bf_tape, ptr -> v`.
- Rewrites `store_mem ptr, v` → `store_byte __bf_tape, ptr, v`.
- Replaces the trailing `ret_void` with `const __bf_ret = 0; ret
  __bf_ret`, changing `main`'s return type from `void` to `i64` so
  the LANG VM AOT chain's entry-point convention (exit code = main's
  return value) is satisfied.

**End-to-end smoke test:** `end_to_end_brainfuck_prints_a_via_lang_aot`
on both Windows + Linux compiles `++++++++[>++++++++<-]>+.` (canonical
"print 'A'") through `lang-aot` and asserts stdout is exactly `"A"`.
This exercises every mechanic LANG75 + LANG76 deliver: pointer shift,
cell mutation, nested loops, the 30000-byte tape, and putchar.
Verified locally on Windows.

**Lib test:** `brainfuck_lowering_inserts_tape_and_byte_ops` asserts
the lowering pass produces the expected IIR shape (alloc_bytes
preamble, no leftover load_mem/store_mem, ret/i64 epilogue) without
needing the linker.

