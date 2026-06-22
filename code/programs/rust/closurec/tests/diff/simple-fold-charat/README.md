# Fixture: `simple-fold-charat`

End-to-end oracle for string-indexing folding (`charCodeAt`/`charAt`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var c = "hello".charCodeAt(0); report(c);` |
| `expected.stdout` | The folded output: `var c=104;report(c);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds the single-integer-index string methods on a string
literal: `"hello".charCodeAt(0)` → `104` (the UTF-16 code unit of `h`), and
likewise `charAt`. JS indexes strings by UTF-16 code unit. Only a non-negative
integer-literal index folds; out-of-range `charCodeAt` (JS `NaN`) and
lone-surrogate `charAt` results are left alone. The same input under
`WHITESPACE_ONLY` keeps `"hello".charCodeAt(0)` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-charat/input/a.js \
    > tests/diff/simple-fold-charat/expected.stdout
```
