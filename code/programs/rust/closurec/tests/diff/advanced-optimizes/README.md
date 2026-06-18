# Fixture: `advanced-optimizes`

End-to-end oracle for `--compilation_level ADVANCED` running the optimization
pipeline (CLOC12.161).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level ADVANCED --js input/a.js` |
| `input/a.js` | A program with a foldable expr, an unused var, and a renameable param |
| `expected.stdout` | `function compute(a){return a + 1};report(compute(7));report(compute(8));` |

ADVANCED used to be a **literal no-op** — it returned the source verbatim. It
now runs the **same typed optimization pipeline as SIMPLE** (it is specified to
be at least as aggressive). So this input is:

| Source | Fate |
|--------|------|
| `var dead = 1 + 2;` | `1 + 2` folds to `3`; the unused `dead` is then removed |
| `function compute(longName) {…}` | kept (called twice, so the single-use inliner leaves it); param `longName` → `a` |
| `report(compute(7)); report(compute(8));` | kept |

`compute` is called twice on purpose: the single-use inliner would otherwise
substitute its body at the lone call site, so two calls keep this fixture's
focus on fold + dead-code removal + rename. Inlining has its own pass-crate
tests.

ADVANCED and SIMPLE produce identical output today; advanced-only passes
(aggressive property/global renaming, cross-module tree-shaking) layer on as
they are implemented.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level ADVANCED \
    --js tests/diff/advanced-optimizes/input/a.js \
    > tests/diff/advanced-optimizes/expected.stdout
```
