# Fixture: `simple-fold-slice`

End-to-end oracle for string-slicing folding (`String#slice`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "abcd".slice(1, 3); report(s);` |
| `expected.stdout` | The folded output: `var s="bc";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `slice` on a string literal with integer-literal
arguments: `"abcd".slice(1, 3)` → `"bc"`. JS `slice` indexes by UTF-16 code
unit over a half-open range `[start, end)`; negative arguments count from the
end, and both ends clamp to `[0, length]`. Zero, one, or two integer-literal
arguments fold; non-integer args, more than two args, an identifier receiver,
or a cut that would split a surrogate pair (yielding a lone surrogate) are left
alone — matching `charAt`'s conservative stance. The same input under
`WHITESPACE_ONLY` keeps `"abcd".slice(1, 3)` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-slice/input/a.js \
    > tests/diff/simple-fold-slice/expected.stdout
```
