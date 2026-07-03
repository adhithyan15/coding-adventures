# Fixture: `simple-fold-radix`

End-to-end oracle for numeric `toString([radix])` folding at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = (255).toString(16); report(s);` |
| `expected.stdout` | The folded output: `var s="ff";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose `constant-fold`
pass folds `Number.prototype.toString` on a non-negative integer literal with a
known radix: `(255).toString(16)` → `"ff"` (and `.toString()` → `"255"`,
`.toString(2)` → `"11111111"`). The radix is the default 10 or a single integer
literal in `2..=36`; a fractional receiver, an out-of-range radix, or a variable
radix pass through. The same input under `WHITESPACE_ONLY` keeps
`(255).toString(16)` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-radix/input/a.js \
    > tests/diff/simple-fold-radix/expected.stdout
```
