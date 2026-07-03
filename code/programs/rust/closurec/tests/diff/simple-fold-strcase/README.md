# Fixture: `simple-fold-strcase`

End-to-end oracle for ASCII string-casing folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "hello".toUpperCase(); report(s);` |
| `expected.stdout` | The folded output: `var s="HELLO";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds the no-argument `.toUpperCase()` / `.toLowerCase()`
methods on an **ASCII** string literal to the cased string (ASCII case mapping
is locale-independent and byte-for-byte equal to JavaScript): `"hello"`
→ `"HELLO"`. Non-ASCII receivers, identifier receivers, the computed form, and
any-argument calls are left alone. The same input under `WHITESPACE_ONLY` keeps
`"hello".toUpperCase()` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-strcase/input/a.js \
    > tests/diff/simple-fold-strcase/expected.stdout
```
