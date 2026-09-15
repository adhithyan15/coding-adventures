## 0.64.0 — 2026-06-12 — Dartmouth BASIC completes the CLR column (LANG-MATRIX LM-C BASIC)

`lang_matrix.rs` greens **Dartmouth BASIC on the real CoreCLR**, completing the CLR
column for every non-Brainfuck language. The CIL backend (`iir-to-cil-bytecode` 0.17.0)
grew the `print_i64` → `Console.WriteLine(int32)` lowering and an I/O-aware launcher that
discards (rather than re-prints) the entry result for a printing program. `run_clr` now
returns the captured `Console` output as well as the parsed integer, so `assert_cell`
compares the one the program's `Expect` cares about (stdout for BASIC, exit-style int for
the expression languages). `Clr` is added to BASIC's `Prog`.

Verified by RUNNING on CoreCLR (dotnet 9.0.x): BASIC `10 PRINT 42` → `Console` `42`
(printed exactly once — no double-print from the launcher). **26 proven matrix cells**
(was 25); the `conformance`, `jvm_emit`, `wasm_emit` and `iir-to-cil-bytecode` suites all
still green. CLR column now green for every language except the deferred Brainfuck.

