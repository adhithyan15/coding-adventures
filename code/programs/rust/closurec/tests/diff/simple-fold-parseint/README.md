# Fixture: `simple-fold-parseint`

End-to-end oracle for global `parseInt` / `parseFloat` folding on string
literals at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | `parseInt("12px")`, `parseInt("FF", 16)`, `parseFloat("3.14abc")`, `parseInt("0x1F")` |
| `expected.stdout` | The folded output: `var a=12,b=255,c=3.14,d=31;report(a,b,c,d);` |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds global `parseInt`/`parseFloat` calls whose first
argument is a string literal (ECMAScript §19.2.5 / §19.2.4) to the numeric
literal V8 produces at runtime:

- `parseInt("12px")` → `12` — the leading digits, trailing `"px"` ignored;
- `parseInt("FF", 16)` → `255` — explicit radix 16;
- `parseFloat("3.14abc")` → `3.14` — the leading decimal, trailing garbage
  ignored;
- `parseInt("0x1F")` → `31` — the auto-detected `0x` hex prefix.

Only the **bare global identifier** folds — a member access like
`window.parseInt(...)` is left for the runtime, as is any call whose result is
`NaN` (`parseInt("")`) or `±Infinity` (`parseFloat("Infinity")`), since
JavaScript has no literal token for those. The same input under
`WHITESPACE_ONLY` keeps the calls intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-parseint/input/a.js \
    > tests/diff/simple-fold-parseint/expected.stdout
```
