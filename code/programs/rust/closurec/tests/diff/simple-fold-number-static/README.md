# `simple-fold-number-static` — static `Number.isInteger/isFinite/isNaN` → boolean

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the ES2015 static numeric predicates
`Number.isInteger(x)`, `Number.isFinite(x)`, and `Number.isNaN(x)` (ECMAScript
§21.1.2.2/.3/.4) to a boolean literal when the single argument is a literal we
can classify.

**Unlike** the *global* `isNaN`/`isFinite`, these do **no** `ToNumber` coercion:
the argument must already be a Number, otherwise the answer is `false`.

| call                     | result  | note                                       |
|--------------------------|---------|--------------------------------------------|
| `Number.isInteger(42)`   | `true`  | a whole number                             |
| `Number.isInteger(3.5)`  | `false` | has a fractional part                      |
| `Number.isInteger(1e21)` | `true`  | huge, but integer-valued (f64 ≥ 2⁵²)       |
| `Number.isFinite(42)`    | `true`  | finite                                     |
| `Number.isNaN(42)`       | `false` | a clean number is not `NaN`                |
| `Number.isInteger("42")` | `false` | a STRING is not a Number — **no coercion** |
| `Number.isFinite(null)`  | `false` | `null` is not a Number                     |

## Soundness

These are STATIC METHOD calls, so they dispatch through the `MemberExpression`
callee arm (alongside `String.fromCharCode`/`fromCodePoint`) — only the bare
global `Number.isX(...)` folds, never a shadowed receiver (`n.isInteger(5)` is
left alone). A NUMBER literal is classified directly from its `f64`
(`is_nan()` / `is_finite()` / `is_finite() && fract()==0.0`); a STRING / BOOLEAN
/ NULL literal is provably not a Number, so all three fold to `false`. Any other
argument (an identifier, array, object) or a call with ≠1 argument has unknown
type at compile time and is left for the runtime.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — seven `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=true;var b=false;var c=true;var d=true;var e=false;var f=false;var g=false;report(a,b,c,d,e,f,g);
  ```

The integration test `tests/diff_simple_fold_number_static.rs` runs the binary
against these flags and asserts byte-exact stdout, the per-binding boolean folds
(including the no-coercion `isInteger("42")` → false and large-integer
`isInteger(1e21)` → true cases), and a regression guard that the typed SIMPLE
pipeline ran (not the WHITESPACE_ONLY fallback): zero `Number.is…(` calls remain.
