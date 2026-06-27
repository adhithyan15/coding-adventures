# `simple-fold-object-keys` — static `Object.keys/values/entries({})` → `[]`

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the static `Object.keys(x)` / `Object.values(x)` /
`Object.entries(x)` (ECMAScript §20.1.2.16/.22/.5) to the empty array literal
`[]` when the single argument is an **empty object literal** `{}`.

| call                   | result      | note                                       |
|------------------------|-------------|--------------------------------------------|
| `Object.keys({})`      | `[]`        | no own enumerable keys                      |
| `Object.values({})`    | `[]`        |                                            |
| `Object.entries({})`   | `[]`        |                                            |
| `Object.keys({a:1})`   | *unfolded*  | non-empty → property values may have effects|
| `Object.keys([])`      | *unfolded*  | an array is declined (conservative)         |

## Soundness

An empty object has no own enumerable keys, and evaluating `{}` has no observable
side effect, so collapsing `Object.keys/values/entries({})` to `[]` is exact and
sound for all three methods. We fold **only** the empty-object-literal case: a
non-empty object literal is declined because its property values (and any
computed keys / spreads) may have side effects that collapsing to `[]` would drop
(and the result is non-empty anyway). An array literal, a primitive
(`Object.keys("ab")` → `["0","1"]`), an identifier, or any non-literal is also
declined. `Object.keys/values/entries` are STATIC METHODS, so they dispatch
through the `MemberExpression` callee arm — only the bare global
`Object.keys(...)` folds, never a shadowed receiver (`o.keys(...)` is left alone).

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=[];var b=[];var c=[];var d=Object.keys({a:1});var e=Object.keys([]);report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_object_keys.rs` runs the binary
against these flags and asserts byte-exact stdout, the three empty-object `[]`
folds, the declined non-empty-object and array calls, and a regression guard that
the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY fallback): exactly two
`Object.` calls (the two declines) remain.
