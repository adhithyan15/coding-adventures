# `simple-fold-number-issafeinteger` — static `Number.isSafeInteger(x)` → boolean

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses a static `Number.isSafeInteger(x)` call (ECMAScript
§21.1.2.5) to a boolean literal.

| call                                       | result   | note                                  |
|--------------------------------------------|----------|---------------------------------------|
| `Number.isSafeInteger(7)`                  | `true`   | small safe integer                    |
| `Number.isSafeInteger(3.5)`                | `false`  | not an integer                        |
| `Number.isSafeInteger(9007199254740991)`   | `true`   | `Number.MAX_SAFE_INTEGER` (2^53−1)    |
| `Number.isSafeInteger(9007199254740992)`   | `false`  | 2^53 — one past the safe range        |
| `Number.isSafeInteger(x)`                  | *unfolded* | identifier — type unknown at compile time |

## Soundness

`Number.isSafeInteger` returns `true` only for a **Number** that is a finite
integer whose magnitude does not exceed 2^53−1 (`Number.MAX_SAFE_INTEGER` =
9007199254740991) — the largest integer the f64 mantissa represents without
sharing its encoding with a neighbouring integer. Like the sibling
`Number.isInteger` / `isFinite` / `isNaN` statics it performs **no** coercion, so
a string / boolean / null literal is `false` outright. We fold a numeric literal
by classifying its value, and a non-number literal to `false`; an identifier or
any non-literal is declined (its type is unknown at compile time). Only the bare
global `Number.isSafeInteger(...)` callee folds (a member access, not a
shadowable free identifier).

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=!0;var b=!1;var c=!0;var d=!1;var e=Number.isSafeInteger(x);report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_number_issafeinteger.rs` runs the
binary against these flags and asserts byte-exact stdout, the four folds (incl.
the critical `9007199254740991` → `true` vs `9007199254740992` → `false`
boundary), the declined identifier, and a regression guard that the typed SIMPLE
pipeline ran (not the WHITESPACE_ONLY fallback): exactly one `Number.isSafeInteger`
call remains.
