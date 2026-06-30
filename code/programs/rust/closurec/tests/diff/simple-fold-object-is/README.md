# `simple-fold-object-is` — static `Object.is(a, b)` → boolean (SameValue)

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses a static `Object.is(a, b)` call (ECMAScript
§20.1.2.13) to a boolean literal when both arguments are primitive literals.

`Object.is` uses the **SameValue** algorithm (§7.2.11), which differs from `===`
in exactly two cases: `Object.is(NaN, NaN)` is `true` and `Object.is(+0, -0)` is
`false`.

| call                  | result     | note                                          |
|-----------------------|------------|-----------------------------------------------|
| `Object.is(1, 1)`     | `true`     | equal numbers                                 |
| `Object.is(0, -0)`    | `false`    | the ±0 distinction (`===` would give `true`)  |
| `Object.is("x", "x")` | `true`     | byte-equal strings                            |
| `Object.is(1, "1")`   | `false`    | different Type → never SameValue              |
| `Object.is(NaN, NaN)` | *unfolded* | `NaN` is the global *identifier*, not a literal |

## Soundness

We fold only when **both** arguments are primitive literals whose values are
known: two number literals compare by SameValue on the f64 (NaN is the same as
NaN; +0 and −0 are distinguished by their sign bit; otherwise `==`), two string
literals byte-equal, two boolean literals, two `null` literals (`true`), and a
mismatch of literal kinds is `false` (SameValue requires the same Type). We
**decline** when either argument is a non-literal — including the bare global
`NaN` (which is an *identifier* read, not a numeric literal, so its value is not
known at compile time) — or when the arity is not two. Only the bare global
`Object.is(...)` callee folds (a member access, not a shadowable free
identifier).

Note the `-0` source token is unary minus applied to `0`; an earlier fold turns
it into the numeric literal −0.0 before this pass runs, so `Object.is(0, -0)`
correctly yields `false`.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=!0;var b=!1;var c=!0;var d=!1;var e=Object.is(NaN,NaN);report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_object_is.rs` runs the binary
against these flags and asserts byte-exact stdout, the four folds (incl. the
headline `Object.is(0, -0)` → `false` ±0 distinction and the `1`/`"1"` type
mismatch), the declined `NaN`-identifier comparison, and a regression guard that
the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY fallback): exactly one
`Object.is` call remains.
