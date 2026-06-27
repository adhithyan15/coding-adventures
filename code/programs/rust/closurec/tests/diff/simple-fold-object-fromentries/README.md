# simple-fold-object-fromentries

End-to-end fixture for folding the static `Object.fromEntries([[k, v], …])`
(ECMAScript §20.1.2.7) to an object literal at `--compilation_level SIMPLE`.

`Object.fromEntries` is the inverse of `Object.entries`: it walks an array of
`[key, value]` pairs and builds a plain object via
CreateDataPropertyOnObject. When the argument is a fully-static array of
2-element `[key, value]` array literals — each key a string/numeric literal,
each value a primitive literal — the result is known at compile time and the
call collapses to an object literal.

## What it checks

| input expression                              | SIMPLE output      | why                                  |
|-----------------------------------------------|--------------------|--------------------------------------|
| `Object.fromEntries([["a", 1], ["b", 2]])`    | `{a:1,b:2}`        | identifier-name keys → bare keys     |
| `Object.fromEntries([[1, "x"]])`              | `{"1":"x"}`        | numeric key → ToString `"1"`, quoted |
| `Object.fromEntries([["a", 1], ["a", 2]])`    | `{a:2}`            | duplicate key → first pos, last value|
| `Object.fromEntries([])`                      | `{}`               | empty input → empty object           |
| `o.fromEntries([["a", 1]])`                    | `o.fromEntries(…)` | declined — not the bare global       |

Expected SIMPLE stdout:

```text
var a={a:1,b:2};var b={"1":"x"};var c={a:2};var d={};var e=o.fromEntries([["a",1]]);report(a,b,c,d,e);
```

Each result flows into `report(...)` so it stays referenced past
remove-unused-vars and the fold is observable.

## Soundness

The fold applies only when EVERY condition holds: exactly one argument that is
an array literal with no holes; every element a 2-element array literal (no
holes) `[key, value]`; the key a string or numeric literal (numeric → ToString);
the value a primitive literal (string / number / boolean / null). Anything else
— wrong arity, a non-array argument, a non-pair element, a non-literal /
boolean / null / identifier key, a non-literal value, a hole, or a shadowed
receiver — is declined and left untouched. Declining is always safe.

## How to run

```bash
cd code/programs/rust/closurec
cargo run -- --compilation_level SIMPLE \
  --js tests/diff/simple-fold-object-fromentries/input/a.js
```

The integration test `tests/diff_simple_fold_object_fromentries.rs` asserts the
byte-exact stdout, the per-binding folds, and that the typed SIMPLE pipeline ran
(not the WHITESPACE_ONLY fallback, under which all five calls would survive).
