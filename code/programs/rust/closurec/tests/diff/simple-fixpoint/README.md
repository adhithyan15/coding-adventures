# Fixture: `simple-fixpoint`

End-to-end oracle for the pass pipeline's **fixed-point iteration**
(CLOC13.F) under `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A single-use function whose call folds only after inlining |
| `expected.stdout` | `log(14);` |

The pipeline now sweeps the pass order repeatedly while any `FixedPoint`
pass still reports a change, so a transform one pass exposes is picked up
by an earlier pass on the next sweep:

| Sweep | What happens |
|-------|--------------|
| 1 | `inline` turns `double(7)` into `7 * 2`; `double` becomes unreferenced and is removed by remove-unused-vars / treeshake |
| 2 | `constant-fold` — which ran *before* inline in sweep 1 and never saw `7 * 2` — folds it to `14` |
| 3 | nothing changes → converged |

Before fixed-point iteration the pipeline ran each pass exactly once and
stopped at `log(7 * 2);`. The result is now `log(14);`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fixpoint/input/a.js \
    > tests/diff/simple-fixpoint/expected.stdout
```
