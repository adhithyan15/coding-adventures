# Fixture: `simple-constant-fold`

End-to-end oracle for `--compilation_level SIMPLE` (CLOC12.155, PR-1).

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | Three `var` declarations with constant arithmetic initializers |
| `expected.stdout` | The folded output: `var sum=3,product=12,nested=14;` |

The SIMPLE level runs the typed-AST optimization pipeline (currently
just the `constant-fold` pass), so `1 + 2` ⇒ `3`, `3 * 4` ⇒ `12`, and
`2 + 3 * 4` ⇒ `14` (operator precedence respected). The same input
under `WHITESPACE_ONLY` keeps `1+2` etc. — see the
`simple_level_whitespace_only_leaves_arithmetic_unfolded` unit test in
`src/run.rs`.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-constant-fold/input/a.js \
    > tests/diff/simple-constant-fold/expected.stdout
```
