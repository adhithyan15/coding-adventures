# Fixture: `simple-fold-trim`

End-to-end oracle for string-trim folding (`String#trim` / `trimStart` /
`trimEnd`) at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `var s = "  hi  ".trim(); report(s);` |
| `expected.stdout` | The folded output: `var s="hi";report(s);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds `trim`/`trimStart`/`trimEnd` on a string literal:
`"  hi  ".trim()` → `"hi"`. The stripped set is the exact ECMAScript
white-space + line-terminator set (U+0009–000D, U+0020, U+00A0, U+1680,
U+2000–200A, U+2028, U+2029, U+202F, U+205F, U+3000, U+FEFF), deliberately
**not** Rust's `char::is_whitespace` — the two disagree on U+0085 (NEL) and
U+FEFF (BOM), so the fold hard-codes the JS set to stay sound. The same input
under `WHITESPACE_ONLY` keeps `"  hi  ".trim()` unfolded.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-trim/input/a.js \
    > tests/diff/simple-fold-trim/expected.stdout
```
