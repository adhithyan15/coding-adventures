# Fixture: `simple-fold-indexof`

End-to-end oracle for string-literal `indexOf` folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var i = "abcabc".indexOf("b"); report(i);` |
| `expected.stdout` | The folded output: `var i=1;report(i);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds the single-argument `String#indexOf` on two string literals:
`"abcabc".indexOf("b")` → `1` (the **UTF-16 code-unit** index of the first
occurrence). An absent needle folds to `-1` and the empty needle to `0`. Only
the one-argument form folds — the `fromIndex` overload
(`"abc".indexOf("b", 1)`) and a non-literal receiver pass through. The same
input under `WHITESPACE_ONLY` keeps `"abcabc".indexOf("b")` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-indexof/input/a.js \
    > tests/diff/simple-fold-indexof/expected.stdout
```
