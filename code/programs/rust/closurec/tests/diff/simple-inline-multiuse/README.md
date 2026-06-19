# Fixture: `simple-inline-multiuse`

End-to-end oracle for **multi-use inlining** (CLOC13.G) under
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A small pure function called at two sites |
| `expected.stdout` | `a(9);b(16);` |

`inline` now substitutes a callee at **all** its call sites (not only when
it is used once), provided every use is an inlinable call and the body fits
the size budget `expr_node_count(body) <= 2 + params.len()` (here `x * x` is
3 nodes; the budget for one parameter is 3). Combined with the pipeline's
fixed-point iteration:

| Sweep | What happens |
|-------|--------------|
| 1 | `sq(3)` → `3 * 3`, `sq(4)` → `4 * 4`; `sq` is now unreferenced and removed by treeshake |
| 2 | `constant-fold` folds `3 * 3` → `9` and `4 * 4` → `16` |
| 3 | nothing changes → converged |

Result: `a(9);b(16);`. A larger body (over the budget) would be left at its
call sites — see the `closure-pass-inline` crate's `does_not_inline_multi_use_large_body`
test.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-inline-multiuse/input/a.js \
    > tests/diff/simple-inline-multiuse/expected.stdout
```
