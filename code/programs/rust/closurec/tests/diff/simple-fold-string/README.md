# Fixture: `simple-fold-string`

End-to-end oracle for global `String(…)` folding on string/integer literals at
`--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | five `String(…)` calls — four foldable, one declined |
| `expected.stdout` | The folded output (see below) |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds a global `String(value)` call whose single argument
is a string or **integer** number literal (ECMAScript §22.1.3.1 → §7.1.17
`ToString`) to a string literal:

- `String(42)` → `"42"` — integer rendered as decimal;
- `String("x")` → `"x"` — a string literal is the identity;
- `String(-3)` → `"-3"` — negative integer;
- `String(255)` → `"255"` — integer;
- `String(0.5)` → left intact (call not folded) — a **fractional** number is
  declined; the surviving `0.5` argument then emits as the minified `.5` (the
  emitter drops a leading fractional zero in value position).

So the folded `expected.stdout` is:

```js
var a="42";var b="x";var c="-3";var d="255";var e=String(.5);report(a,b,c,d,e);
```

Fractional numbers are deliberately **not** folded: Rust's `f64::to_string` and
V8's `Number::toString` are both shortest-round-trip but can break an exact
binary tie in opposite directions (a last-digit-off-by-one), so folding them
could silently change the program. Only the **bare global identifier** folds — a
member access like `window.String(...)` is left for the runtime, as is any
non-string/non-integer argument. The same input under `WHITESPACE_ONLY` keeps
every call intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-string/input/a.js \
    > tests/diff/simple-fold-string/expected.stdout
```
