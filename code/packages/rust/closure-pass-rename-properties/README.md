# coding-adventures-closure-pass-rename-properties

Aggressive property renaming pass for the Closure Compiler clone
(**ADVANCED**-only) — Closure Compiler's `RENAME_PROPERTIES` in miniature.
Consistently shortens program-private object **property names**. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md).

## What it does

```js
// before
obj.computeTotal = function () {};
widget.computeTotal();
var cfg = { renderMode: 1 };

// after rename-properties (ADVANCED)
obj.a = function () {};
widget.a();
var cfg = { b: 1 };
```

Property access is *by name*, so renaming a name at **every** occurrence
(dotted `obj.x` + unquoted `{ x: … }`) is semantics-preserving no matter
which objects carry it.

## Soundness — the externs contract

A property name is renamed only when it:

1. appears dotted / unquoted;
2. is NOT quoted via a computed string member (`obj["x"]` — the bridge
   preserves this; the author quoted it to mean "external/dynamic");
3. is not a `BUILTIN_PROPERTIES` (a bundled ECMAScript default-externs
   substitute — `length`, `prototype`, `toString`, `push`, …);
4. is not in the externs do-not-rename set; and
5. is longer than one character.

Each renameable property gets a **distinct** fresh name. The fresh name
only avoids other property names + the built-ins + the externs set
(property names are their own namespace; a property may become `a` even
when a variable `a` exists).

### Honest limitations

- The built-in list covers ECMAScript but **not the DOM/host** — host
  property names (`innerHTML`, `addEventListener`, …) must be supplied via
  `--externs` or they will be renamed.
- The parser bridge currently collapses a quoted object key `{ "x": 1 }`
  to an identifier key, so object-key quoting is **not** a usable
  do-not-rename signal (only computed-member quoting `obj["x"]` is) —
  protect such names via externs.
- Dynamic computed access `obj[runtimeString]` is the author's contract
  responsibility, exactly as in Closure.

## Status

This crate is the **algorithmic core**; wiring it into ADVANCED (collect
externs property names + decide the safe-by-default policy) is a
deliberate follow-up.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` and the typed AST.

Dev-deps: `coding-adventures-javascript-tokens`,
`coding-adventures-javascript-parser`, `coding-adventures-closure-emitter`,
`coding-adventures-type-sidecar`, `coding_adventures_correlation_vector`.
