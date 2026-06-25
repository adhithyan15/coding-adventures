# Fixture: `simple-fold-at`

End-to-end oracle for string-index folding (`String#at`, negative-from-end) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "abcde".at(-2); report(s);` |
| `expected.stdout` | The folded output: `var s="d";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `at` on a string literal with an integer-literal
index: `"abcde".at(-2)` → `"d"`. Unlike `charAt`, a **negative** index counts
from the end (`len + i`), and indexing is by UTF-16 code unit. An out-of-range
index is `undefined` in JS (no literal, so the call is left untouched, rather
than the `""` that `charAt` would give); a fractional/non-literal index and a
lone-surrogate result also pass through to the runtime. The same input under
`WHITESPACE_ONLY` keeps `"abcde".at(-2)` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-at/input/a.js \
    > tests/diff/simple-fold-at/expected.stdout
```
