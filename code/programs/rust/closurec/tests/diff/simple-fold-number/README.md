# Fixture: `simple-fold-number`

End-to-end oracle for global `Number("…")` folding on string literals at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | seven `Number("…")` calls — six foldable, one declined |
| `expected.stdout` | The folded output (see below) |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds a global `Number(string)` call whose single argument
is a string literal (ECMAScript §21.1.1.1 → §7.1.4.1.1 `StringToNumber`) to the
numeric literal V8 produces at runtime. Unlike `parseInt`/`parseFloat`, which
read a leading *prefix*, `Number` is **total** — the whole trimmed string must
be numeric or the result is `NaN`:

- `Number("42")` → `42` — plain decimal;
- `Number("")` → `0` — the empty string coerces to `+0`, **not** `NaN`;
- `Number("  3.5 ")` → `3.5` — surrounding whitespace is trimmed;
- `Number("0x1F")` → `31` — hex (no sign permitted);
- `Number("0b101")` → `5` — binary;
- `Number("0o17")` → `15` — octal;
- `Number("abc")` → left intact — not numeric, so the runtime result is `NaN`,
  which has no literal token to substitute.

So the folded `expected.stdout` is:

```js
var a=42,b=0,c=3.5,d=31,e=5,f=15,g=Number("abc");report(a,b,c,d,e,f,g);
```

Only the **bare global identifier** folds — a member access like
`window.Number(...)` is left for the runtime, as is any call whose result is
`NaN` or `±Infinity` (`Number("Infinity")`). The same input under
`WHITESPACE_ONLY` keeps every call intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-number/input/a.js \
    > tests/diff/simple-fold-number/expected.stdout
```
