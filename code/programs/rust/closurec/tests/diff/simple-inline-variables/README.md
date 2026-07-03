# Fixture: `simple-inline-variables`

End-to-end oracle for the `inline-variables` (constant-propagation) pass
in `--compilation_level SIMPLE` (CLOC13.H).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A top-level `const` bound to a literal, used in two expressions |
| `expected.stdout` | `total(base * 2);margin(3);` |

The SIMPLE pipeline is now
`constant-fold → fold-control-flow → dce → inline → inline-variables →
remove-unused-vars → treeshake → rename`. The new pass propagates a
top-level `const` whose value is a literal to its use sites:

| Sweep | What happens |
|-------|--------------|
| 1 | `RATE` (`const = 2`) is propagated → `base * 2` and `2 + 1`; the `const RATE = 2;` declaration is now unreferenced and removed by `remove-unused-vars` |
| 2 | `constant-fold` folds `2 + 1` → `3` |
| 3 | nothing changes → converged |

`base` is a free variable, so `base * 2` cannot be folded further. Only
`const` bound to a *literal* is propagated — `let`/`var` (reassignable)
and non-literal initializers are left alone. See the
`closure-pass-inline-variables` crate for the full (conservative) rules.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-inline-variables/input/a.js \
    > tests/diff/simple-inline-variables/expected.stdout
```
