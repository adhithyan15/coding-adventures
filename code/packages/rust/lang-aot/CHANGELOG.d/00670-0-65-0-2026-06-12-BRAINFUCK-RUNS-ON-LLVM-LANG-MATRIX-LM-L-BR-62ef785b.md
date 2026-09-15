## 0.65.0 — 2026-06-12 — Brainfuck runs on LLVM (LANG-MATRIX LM-L Brainfuck)

`lang_matrix.rs` greens **Brainfuck on the LLVM backend** — the first cell of the
deferred Brainfuck row, and the last code-gen gap for that language (only the VM/JIT
columns remain). Verified by RUNNING `++++++++[>++++++++<-]>+.` on real `clang`: it
prints `A`.

The fix is split across two layers, neither of which touches the McCarthy-critical
`iir-to-llvm` stack-slot allocator:

- **`lower_brainfuck_for_aot` (this crate) — Step 5: widen narrow hints to `i64`.**
  The Brainfuck frontend emits `u8` (cells) / `u32` (pointer) `type_hint`s. The
  AOT/LLVM pass now widens every narrow-integer hint to `i64` so the value model is a
  uniform machine word. Byte width survives **only at the tape boundary**: `load_byte`
  zero-extends the 8-bit cell to `i64` and `store_byte` truncates back, so cell
  wrap-around (`255 + 1 == 0`) stays correct. This is what lets `iir-to-llvm` — which
  promotes any reassigned variable (BF's `ptr`/`v`/`c`/`k`) to an `alloca i64` slot —
  consume Brainfuck without a width mismatch (`'%__ld' defined with type 'i64' but
  expected 'i8'`). It is done in this BF-specific pass, **not the frontend**, so the
  frontend's `u8`/`u32` hints still reach `vm-core`/`jit-core`, whose `specialise` step
  keys CIR opcode widths (`add_u8`/`add_u32`) off them — the brainfuck-iir-compiler VM
  and JIT paths are untouched. Native AOT is unaffected: its byte ops already ignore
  the hint and its arithmetic runs in 64-bit registers regardless.
- **`iir-to-llvm` 0.9.0** grew the byte-tape ops (`alloc_bytes`/`load_byte`/`store_byte`)
  and the `putchar`/`getchar` libc builtins, plus a slot-dest SSA rename (see that
  crate's changelog).

`tests/lang_matrix.rs` adds `Llvm` to the Brainfuck `Prog`; two new `--lib` tests cover
the i64 widening (`brainfuck_lowering_widens_narrow_hints_to_i64`,
`is_narrow_int_hint_classifies_widths`).

