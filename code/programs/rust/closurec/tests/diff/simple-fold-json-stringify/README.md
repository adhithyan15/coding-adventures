# `simple-fold-json-stringify` — static `JSON.stringify(…)` → string literal

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the static `JSON.stringify(x)` (ECMAScript §25.5.2)
to a string literal for the primitive literal arguments whose JSON text can be
rendered exactly.

| call                    | result      | note                                    |
|-------------------------|-------------|-----------------------------------------|
| `JSON.stringify(42)`    | `"42"`      | a number's `ToString` is its JSON form  |
| `JSON.stringify(-7)`    | `"-7"`      |                                          |
| `JSON.stringify(true)`  | `"true"`    |                                          |
| `JSON.stringify(null)`  | `"null"`    |                                          |
| `JSON.stringify("x")`   | *unfolded*  | JSON string escaping left to the runtime |
| `JSON.stringify(3.5)`   | *unfolded*  | fractional → `fold_string_of_number` None|

## Soundness

`JSON.stringify` is a STATIC METHOD, so it dispatches through the
`MemberExpression` callee arm (alongside the `String.from*` and
`Number.isX`/`parseX` statics) — only the bare global `JSON.stringify(...)` folds,
never a shadowed receiver. We fold only the **single-argument** form (a
`replacer`/`space` second argument can change the result — a replacer function is
invoked even on a primitive), and only for a NUMBER / BOOLEAN / NULL literal. The
number case reuses `fold_string_of_number`, which declines fractional values and
magnitudes ≥ 2⁵³ (whose shortest-decimal / exponential spelling could diverge
from V8). A STRING literal is declined (JSON escaping is subtle); array/object
literals are declined (side effects + recursion). The folded text is pure ASCII
(digits / `true` / `false` / `null`), so it needs no escaping.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — six `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a="42";var b="-7";var c="true";var d="null";var e=JSON.stringify("x");var f=JSON.stringify(3.5);report(a,b,c,d,e,f);
  ```

The integration test `tests/diff_simple_fold_json_stringify.rs` runs the binary
against these flags and asserts byte-exact stdout, the per-binding string folds
(`42`/`-7`/`true`/`null`), the declined string and fractional calls, and a
regression guard that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY
fallback): exactly two `JSON.stringify(` calls (the two declines) remain.
