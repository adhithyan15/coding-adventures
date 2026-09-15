## 0.54.0 — 2026-06-11 — cross-language platform-matrix harness (LANG-MATRIX LM0)

First slice of the `LANG-PLATFORM-MATRIX` campaign (every language on every non-BEAM
backend). New `tests/lang_matrix.rs`: a per-language program battery (`Expect::Exit`
for the expression languages Twig/Nib/Oct/ALGOL, `Expect::Stdout` for the I/O
languages Brainfuck/BASIC) and a host-gated **native-AOT** runner. Proven by RUNNING
— all six non-Lisp languages compile to a host executable and run with the expected
result: Twig→42, Nib `double(21)`→42, Oct→0, ALGOL `17 mod 5`→2, Brainfuck→stdout `A`,
Dartmouth BASIC `PRINT 42`→stdout `42`. native-AOT is now a uniformly-green floor for
the matrix. Also fixed the stale `Language` enum doc comments (DartmouthBasic/Oct were
wrongly labelled "placeholder / no Rust frontend"; all six are wired into
`compile_source_to_iir`).

Ground truth the probe established (and recorded in the spec): the **VM and JIT are
McCarthy-specialized** — `mccarthy_lisp_vm::run` rejects ordinary `add`/`mul`/`cmp_*`/
`mod` and the I/O ops, so those two columns are real op-coverage work, while the
code-gen backends (native/LLVM/WASM/JVM/CLR) are general and need mostly conformance
tests.

