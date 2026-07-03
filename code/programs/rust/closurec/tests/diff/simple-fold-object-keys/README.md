# `simple-fold-object-keys` — static `Object.keys/values/entries` folding

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass folds the static `Object.keys` / `Object.values` /
`Object.entries` (ECMAScript §20.1.2.16/.22/.5) calls below:

| call                     | result      | note                                          |
|--------------------------|-------------|-----------------------------------------------|
| `Object.keys({})`        | `[]`        | empty object → no own keys                     |
| `Object.values({})`      | `[]`        | empty object                                   |
| `Object.entries({})`     | `[]`        | empty object                                   |
| `Object.keys({a:1,b:2})` | `["a","b"]` | non-empty → array of own string keys           |
| `Object.values({a:1})`   | *unfolded*  | no non-empty `values` fold yet                 |
| `Object.keys({1:"x"})`   | *unfolded*  | integer-index key enumerates first → reorders  |
| `Object.keys([])`        | *unfolded*  | an array is declined (conservative)            |

## Soundness

An empty object has no own enumerable keys and evaluating `{}` has no observable
side effect, so collapsing `Object.keys/values/entries({})` to `[]` is exact.

For a NON-EMPTY object literal, `Object.keys` folds to the array of its own
enumerable string keys. The fold's soundness conditions are IDENTICAL to
`Object.entries`, NOT weaker even though the value is dropped: evaluating the
source object literal still runs each value expression, so `Object.keys({a:foo()})`
and `Object.keys({a:x})` (undeclared `x`) must be left untouched. Every property
must therefore be a plain data property with a side-effect-free primitive literal
value and a non-`__proto__`, non-array-index key; getters/setters/methods,
computed keys, and any canonical integer-index key (which would reorder the
result, as it enumerates ahead of string keys) decline the whole fold. Duplicate
keys collapse to a single first-position entry.

`Object.keys/values/entries` are STATIC METHODS, so they dispatch through the
`MemberExpression` callee arm — only the bare global `Object.keys(...)` folds,
never a shadowed receiver (`o.keys(...)` is left alone).

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — seven `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=[];var b=[];var c=[];var d=["a","b"];var e=Object.values({a:1});var f=Object.keys({1:"x"});var g=Object.keys([]);report(a,b,c,d,e,f,g);
  ```

The integration test `tests/diff_simple_fold_object_keys.rs` runs the binary
against these flags and asserts byte-exact stdout, the empty-object `[]` folds,
the non-empty `Object.keys` key-array fold, the three declined calls, and a
regression guard that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY
fallback): exactly three `Object.` calls (the three declines) remain.
