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

## Building the externs property boundary — `collect_property_names`

The pass protects a `do_not_rename` set, but a driver needs to *build* that
set from the user's `--externs` files. The crate exposes:

```rust
pub fn collect_property_names(program: &Program) -> HashSet<String>
```

It returns **every property name appearing anywhere** in a program — dotted
(`el.innerHTML`), quoted (`obj["data-id"]`), and object keys (unquoted
`{ onload: f }` and quoted `{ "aria-label": s }`). A driver parses each
`--externs` file, walks it through `collect_property_names`, unions the
results, and hands that set to `RenamePropertiesPass::new`. It over-collects
deliberately: any property an externs file names is external and must be
preserved, and forgoing a rename is never a miscompile. Dynamic computed
keys (`obj[runtimeExpr]`) contribute nothing — there is no static name.

This is the property-namespace twin of collecting an externs file's
top-level variable/function names (the value-namespace boundary that gates
`rename-globals`).

## Status

This crate is the **algorithmic core** plus the externs-boundary collector.
It is wired into closurec's ADVANCED level under a **safe-by-default policy**:
property renaming runs only when the user passes at least one `--externs`
file (opting into the externs contract and supplying the host/DOM property
boundary). Without `--externs`, ADVANCED leaves property names untouched —
the built-in list omits the DOM, so renaming by default would miscompile
browser code.

## Dependency whitelist

- `coding-adventures-closure-pass-pipeline` — `Pass` trait + types.
- `coding-adventures-javascript-ast` — `Program` and the typed AST.

Dev-deps: `coding-adventures-javascript-tokens`,
`coding-adventures-javascript-parser`, `coding-adventures-closure-emitter`,
`coding-adventures-type-sidecar`, `coding_adventures_correlation_vector`.
