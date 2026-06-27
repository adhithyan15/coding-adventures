# `simple-fold-array-isarray` — static `Array.isArray(…)` → boolean

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the static `Array.isArray(x)` (ECMAScript §22.1.2.2)
to a boolean literal for the literal argument shapes whose evaluation has **no
observable side effect to drop**.

| call                   | result      | note                                       |
|------------------------|-------------|--------------------------------------------|
| `Array.isArray([])`    | `true`      | the only literal that IS an Array          |
| `Array.isArray({})`    | `false`     | an object is not an Array                   |
| `Array.isArray("x")`   | `false`     | a string is not an Array                    |
| `Array.isArray(42)`    | `false`     | a number is not an Array                    |
| `Array.isArray(null)`  | `false`     | `null` is not an Array                      |
| `Array.isArray([1,2])` | *unfolded*  | non-empty → folding would drop element eval |

## Soundness

`Array.isArray` is a STATIC METHOD, so it dispatches through the
`MemberExpression` callee arm (alongside the `String.from*` and
`Number.isX`/`parseX` statics) — only the bare global `Array.isArray(...)` folds,
never a shadowed receiver (`a.isArray(...)` is left alone). A **non-empty** array
or object literal is DECLINED, because replacing the call with a boolean would
discard the element/property expressions and drop any side effect they evaluate
(`Array.isArray([f()])` must still call `f`). An identifier or any other
non-literal argument has unknown type at compile time and is also left for the
runtime.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — six `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=true;var b=false;var c=false;var d=false;var e=false;var f=Array.isArray([1,2]);report(a,b,c,d,e,f);
  ```

The integration test `tests/diff_simple_fold_array_isarray.rs` runs the binary
against these flags and asserts byte-exact stdout, the per-binding boolean folds
(including the empty-array `true` and primitive/object `false` cases), the
declined non-empty array, and a regression guard that the typed SIMPLE pipeline
ran (not the WHITESPACE_ONLY fallback): exactly one `Array.isArray(` call (the
declined non-empty array) remains.
