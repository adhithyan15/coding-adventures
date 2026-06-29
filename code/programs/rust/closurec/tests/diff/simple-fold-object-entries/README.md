# simple-fold-object-entries

End-to-end fixture for folding the static `Object.entries({k: v, …})`
(ECMAScript §20.1.2.5) to an array of `[key, value]` pair literals at
`--compilation_level SIMPLE`.

`Object.entries` is the inverse of `Object.fromEntries`: it lists an object's
own enumerable string-keyed entries. When the argument is a fully-static object
literal of plain data properties with primitive-literal values, the result is
known at compile time and the call collapses to an array literal.

## What it checks

| input expression               | SIMPLE output             | why                                  |
|--------------------------------|---------------------------|--------------------------------------|
| `Object.entries({a: 1, b: 2})` | `[["a",1],["b",2]]`       | own string entries, source order     |
| `Object.entries({x: "hi"})`    | `[["x","hi"]]`            | single entry                         |
| `Object.entries({})`           | `[]`                      | empty case (pre-existing fold)       |
| `Object.entries({1: "x"})`     | `Object.entries({1:"x"})` | declined — integer-index key reorders|
| `Object.entries({__proto__:1})`| `Object.entries(…)`       | declined — prototype setter, not own |
| `o.entries({a: 1})`            | `o.entries({a:1})`        | declined — not the bare global       |

Expected SIMPLE stdout:

```text
var a=[["a",1],["b",2]];var b=[["x","hi"]];var c=[];var d=Object.entries({1:"x"});var e=Object.entries({__proto__:1});var f=o.entries({a:1});report(a,b,c,d,e,f);
```

Each result flows into `report(...)` so it stays referenced past
remove-unused-vars and the fold is observable.

## Soundness

The fold applies only when every property is a plain data property (`k: v` — no
getter, setter, method, or computed key) with a primitive-literal value (string
/ number / boolean / null). It declines a `"__proto__"` key (whose object-literal
form is the §B.3.1 prototype setter, creating no own property), any canonical
array-index key (enumerated ahead of string keys, which would reorder the
result), any non-literal value (including a shorthand `{x}`), a non-global
receiver, and any arity ≠ 1. Duplicate keys collapse to one entry (first
position, last value). Declining is always safe.

## How to run

```bash
cd code/programs/rust/closurec
cargo run -- --compilation_level SIMPLE \
  --js tests/diff/simple-fold-object-entries/input/a.js
```

The integration test `tests/diff_simple_fold_object_entries.rs` asserts the
byte-exact stdout, the per-binding folds, and that the typed SIMPLE pipeline ran
(not the WHITESPACE_ONLY fallback, under which all six calls would survive).
