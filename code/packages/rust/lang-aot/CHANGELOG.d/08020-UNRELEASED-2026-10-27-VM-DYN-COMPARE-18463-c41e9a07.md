## Unreleased — #18463 — Twig's dynamic comparisons and `pair?` on the VM and JIT columns

- Three `lang_matrix` Twig rows, declared on `Vm` and `Jit` only:
  - `(define (f x) (if (< x 0) 42 7)) (f -5)` → 42 — the #18463 program.
  - `(define (f x) (if (<= x 0) (if (> x -10) (if (>= x -5) 42 1) 2) 3)) (f -5)` → 42,
    which runs the other three orderings with a distinct wrong answer for each.
  - `(define (f x) (if (pair? x) 40 2)) (+ (f (cons 1 2)) (f 5))` → 42, where a
    `pair?` stuck at true gives 80 and one stuck at false gives 4.
- The comparison on an un-annotated parameter reaches vm-core as
  `call_builtin "<"` (and `pair?` survives the structural heap lowering as a
  `call_builtin`). vm-core now registers both by default, so the runners
  change in no way. The Jit cells run `f` on the interpreter tier (it is not
  FullyTyped), so they prove the JIT pipeline's fallback, not JIT bytecode.
- The `(f -5)` row overlaps #18453's PR, which adds the same program on
  NativeAot / Llvm / Wasm / Jvm with the signed-compare fix those columns
  need; fold the two rows once both land.
- `feature_coverage_doc_counts_match_programs_source`: Twig 49 → 52 rows,
  392 → 398 cells; `LANG-VM-FEATURE-COVERAGE.md` updated to match.
