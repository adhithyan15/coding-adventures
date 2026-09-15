## 0.179.0 — 2026-07-06 — BA string INPUT: `INPUT A$` reads a runtime string (VM/JIT)

A new Dartmouth BASIC matrix cell — `10 INPUT A$ / 20 PRINT A$ / 30 END` — proves
that `INPUT A$` reads a whole stdin line **as the string value itself**, not as a
number to parse (`INPUT X`) nor as a selector between compile-time literals (the
E4-dyn foothold's `INPUT N`).  `A$` holds bytes that never appear in the program
source, so the compiler cannot fold it: this is the first matrix proof of a
runtime string that **originates at the input boundary**.

- Frontend: `dartmouth-basic-iir-compiler` 0.36.0 lowers `INPUT A$` to
  `call_builtin "input_str"` (a `str`-typed sibling of `input_i64`) + `mov` into
  the string slot; `PRINT A$` consumes it via the shared E4 `print_str` op.
- Harness (`tests/lang_matrix.rs`): a new `input_str` closure — registered on the
  VM, the JIT's interpreter-fallback VM tier, and the `GenericCirJit` backend —
  reads a line from the shared stdin buffer via a new `drain_stdin_line` helper
  and returns a tagged `vm_core::Value::Str`.  The cell lists `[Vm, Jit]`; its
  stdin `"OK\n"` is registered in `program_stdin`.  Stdin `"OK"` → `A$ = "OK"` →
  prints `OK`.  Wiring the four subprocess/WASM columns' host read-a-line
  primitive (`__twig_input_str` / `env.__input_str` / `readLine` /
  `Console.ReadLine`) is the next slice of this arc.

