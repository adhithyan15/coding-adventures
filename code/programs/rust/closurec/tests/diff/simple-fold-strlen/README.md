# Fixture: `simple-fold-strlen`

End-to-end oracle for string-literal `.length` folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var n = "hello".length; report(n);` |
| `expected.stdout` | The folded output: `var n=5;report(n);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds the `.length` of a string literal to its
UTF-16 code-unit count (JS `String#length` semantics): `"hello".length`
→ `5`. The same input under `WHITESPACE_ONLY` keeps `"hello".length`
unfolded (that level never runs the typed pipeline).

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-strlen/input/a.js \
    > tests/diff/simple-fold-strlen/expected.stdout
```
