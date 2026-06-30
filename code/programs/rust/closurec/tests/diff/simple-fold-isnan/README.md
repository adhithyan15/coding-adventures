# `simple-fold-isnan` — global `isNaN` / `isFinite` constant-fold → boolean

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the global numeric predicates `isNaN(x)` and
`isFinite(x)` (ECMAScript §19.2.3 / §19.2.2) to a boolean literal when the single
argument is a string- or number-literal.

Both coerce the argument with `ToNumber`, then classify the result: `isNaN` is
`true` exactly when it is `NaN`; `isFinite` is `true` exactly when it is neither
`NaN` nor `±Infinity`. Unlike `Number(...)` (which declines the values it cannot
render as an exact literal), **no shape declines** here — every string has a
well-defined NaN / Infinity / finite class:

| call                   | result  | note                                     |
|------------------------|---------|------------------------------------------|
| `isNaN("abc")`         | `true`  | `"abc"` coerces to `NaN`                 |
| `isNaN("42")`          | `false` | a clean number                          |
| `isNaN(" ")`           | `false` | `ToNumber(" ")` is `+0`, not `NaN`       |
| `isFinite("1e3")`      | `true`  | `1000` is finite                         |
| `isFinite("Infinity")` | `false` | `+Infinity` is not finite                |
| `isFinite("abc")`      | `false` | `NaN` is not finite                      |
| `isFinite(0)`          | `true`  | a finite number literal                  |

## Soundness

`isNaN` / `isFinite` are *free identifiers* — a local binding could shadow the
global — so we fold the **bare identifier** only, never a member access
(`window.isNaN` is left alone). The `ToNumber` coercion is computed by the
`js_to_number` helper, which returns the exact `f64` (including `NaN` and
`±Infinity`); the fold only ever reads its `is_nan()` / `is_finite()`
classification — never emits the number — so even a value beyond the
exact-integer range is classified correctly. Booleans are emitted in the
compiler's canonical literal form (`true` / `false`).

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — seven `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=!0;var b=!1;var c=!1;var d=!0;var e=!1;var f=!1;var g=!0;report(a,b,c,d,e,f,g);
  ```

The integration test `tests/diff_simple_fold_isnan.rs` runs the binary against
these flags and asserts byte-exact stdout, the per-binding boolean folds
(including the `ToNumber(" ")=+0` and `"Infinity"` cases), and a regression guard
that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY fallback): zero
`isNaN(` and zero `isFinite(` calls remain.
