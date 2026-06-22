# Fixture: `simple-fold-pad`

End-to-end oracle for string-padding folding (`String#padStart` / `padEnd`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "5".padStart(3, "0"); report(s);` |
| `expected.stdout` | The folded output: `var s="005";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `padStart`/`padEnd` on a string literal with an
integer-literal target length and a string-literal pad (default a single
space): `"5".padStart(3, "0")` → `"005"`. The pad is repeated and truncated to
the shortfall, measured in UTF-16 code units. A string already at or over the
target is returned unchanged. A non-integer target, a non-literal pad, a target
over the optimizer's 100 000-code-unit cap (a denial-of-service guard), or a
fill that would split a surrogate pair all pass through. The same input under
`WHITESPACE_ONLY` keeps `"5".padStart(3, "0")` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-pad/input/a.js \
    > tests/diff/simple-fold-pad/expected.stdout
```
