## Unreleased — untyped Twig integers compare signed on the native column (#18483)

`lang_matrix` gains five Twig rows that compare negative values, each on
NativeAot, LLVM, WASM, JVM, VM and JIT:

| row | expected exit |
|---|---|
| `(if (< -8 100) 1 2)` | 1 |
| `(if (> -1 0) 1 2)` | 2 |
| `(if (<= -5 -5) (if (<= -5 0) 1 3) 2)` | 1 |
| `(if (< (- 0 8) 0) 1 2)` | 1 |
| `(define (neg7) (- 0 7)) (if (< (neg7) 0) 1 2)` | 1 |

Before the `twig-aot` fix, all five failed on NativeAot (exits 2, 1, 3, 2
and 2) because the comparison was specialised `cmp_*_bool`, which the x86-64
backend compares unsigned. The LLVM, WASM, JVM, VM and JIT cells already
passed: `iir-to-llvm` maps a `bool` comparison hint to a signed `i64`
`icmp`, and the other columns never read the hint's signedness. `(<= -5 -5)`
alone is true either way, so its row also asks `(<= -5 0)`.

The rows are not declared on CLR or BEAM: neither toolchain was available
where they were added. `feature_coverage_doc_counts_match_programs_source`
and `LANG-VM-FEATURE-COVERAGE.md` move Twig from 49 rows / 392 cells to
54 / 422.
