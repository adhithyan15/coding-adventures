# Fixture: `simple-fold-repeat`

End-to-end oracle for string-repeat folding (`String#repeat`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "ab".repeat(3); report(s);` |
| `expected.stdout` | The folded output: `var s="ababab";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `repeat` on a string literal with a non-negative
integer-literal count: `"ab".repeat(3)` → `"ababab"`. The receiver is
concatenated `count` times, so — unlike `slice` — there is no surrogate-pair
hazard. A negative count (JS `RangeError`), a fractional/non-literal count, or a
result over the optimizer's 100 000-code-unit cap (a denial-of-service guard
against materializing a huge literal at compile time) all pass through. The same
input under `WHITESPACE_ONLY` keeps `"ab".repeat(3)` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-repeat/input/a.js \
    > tests/diff/simple-fold-repeat/expected.stdout
```
