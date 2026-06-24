# Fixture: `simple-fold-concat`

End-to-end oracle for string-concat folding (`String#concat`) at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "foo".concat("bar", "baz"); report(s);` |
| `expected.stdout` | The folded output: `var s="foobarbaz";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `concat` when the receiver and every argument are
string literals: `"foo".concat("bar", "baz")` → `"foobarbaz"`. Concatenating
valid strings can only ever produce valid UTF-16, so — unlike `slice` — there
is no surrogate-pair hazard. A non-string argument (which JS would coerce via
`ToString`, e.g. `"a".concat(1)` → `"a1"`), a non-literal argument, or a result
over the optimizer's 100 000-code-unit cap (a denial-of-service guard against
materializing a huge literal at compile time) all pass through. The same input
under `WHITESPACE_ONLY` keeps `"foo".concat("bar", "baz")` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-concat/input/a.js \
    > tests/diff/simple-fold-concat/expected.stdout
```
