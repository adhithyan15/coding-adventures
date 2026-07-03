# Fixture: `simple-fold-boolean`

End-to-end oracle for global `Boolean(…)` folding on string/number literals at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | six `Boolean(…)` calls — five foldable, one declined |
| `expected.stdout` | The folded output (see below) |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds a global `Boolean(value)` call whose single argument
is a string or number literal (the `ToBoolean` coercion, ECMAScript §7.1.2) to a
boolean literal:

- `Boolean("")` → `false` — the empty string is the only falsy string;
- `Boolean("x")` → `true`;
- `Boolean("0")` → `true` — a **non-empty** string is truthy even if it looks
  falsy;
- `Boolean(0)` → `false` — `0` (and `-0`) is falsy;
- `Boolean(1)` → `true`;
- `Boolean(z)` → left intact — an identifier needs its runtime value.

So the folded `expected.stdout` is:

```js
var a=!1;var b=!0;var c=!0;var d=!1;var e=!0;var f=Boolean(z);report(a,b,c,d,e,f);
```

Only the **bare global identifier** folds — a member access like
`window.Boolean(...)` is left for the runtime, as is any argument that is not a
string or number literal (a boolean, `null`, an identifier, a second argument).
The same input under `WHITESPACE_ONLY` keeps every call intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-boolean/input/a.js \
    > tests/diff/simple-fold-boolean/expected.stdout
```
