# Fixture: `simple-rename`

End-to-end oracle for the `rename` pass in `--compilation_level SIMPLE`
(CLOC12.160).

| File | Role |
|------|------|
| `flags.txt` | `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | A called leaf function with descriptive parameter names |
| `expected.stdout` | `function distance(a,b){return a * a + b * b};distance(3,4);` |

The SIMPLE pipeline is now
`constant-fold → fold-control-flow → dce → inline → remove-unused-vars →
treeshake → rename`. The final pass shortens the parameters of leaf
functions:

| Source | Fate |
|--------|------|
| function name `distance` | kept — top-level, potentially externally visible |
| param `horizontal` | renamed to `a` |
| param `vertical` | renamed to `b` |

`distance(3, 4)` keeps the function past `treeshake`. The same input under
`WHITESPACE_ONLY` keeps the full parameter names. See the
`closure-pass-rename` crate for the full (conservative) rename rules —
property names, free globals, redeclared params, and non-leaf functions are
all left untouched.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-rename/input/a.js \
    > tests/diff/simple-rename/expected.stdout
```
