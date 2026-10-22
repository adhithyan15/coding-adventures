# coding-adventures-closure-scope-analyzer

Lexical scope and symbol-table analyzer for the Closure Compiler
clone.

## What it does

Given a `Program` from `coding-adventures-javascript-ast`, produces
a `ScopeAnalysis` — the scope tree, the list of declared bindings,
and the list of identifier references with their resolved bindings.
The analysis is consumed by the five Phase-1 optimisation passes
(`rename`, `inline`, `treeshake`, `collapse-properties`,
`remove-unused-vars`) so they don't each have to walk the AST to
build their own symbol tables.

## Where it sits in the stack

```
                   ┌───────────────────────────────────┐
                   │     coding-adventures-closurec     │   (CLI)
                   └───────────────────────────────────┘
                                    │
                ┌───────────────────┴───────────────────┐
                │  coding-adventures-closure-pass-pipeline  │
                └───────────────────┬───────────────────┘
                                    │
   ┌────────────────────────────────┼────────────────────────────────┐
   │                                │                                │
┌──┴──┐ ┌──────────┐ ┌──────┐ ┌─────┴──────┐ ┌────────────┐ ┌────────┴─────────┐
│const│ │fold-ctl- │ │ dce  │ │  rename   │ │   inline   │ │  remove-unused-  │
│fold │ │  flow    │ │      │ │            │ │            │ │     vars         │
└─────┘ └──────────┘ └──────┘ └─────┬──────┘ └─────┬──────┘ └────────┬─────────┘
                                    │              │                 │
                                    └──────────────┼─────────────────┘
                                                   │  (also: treeshake,
                                                   │   collapse-properties)
                                                   ▼
                          ┌─────────────────────────────────────────┐
                          │ coding-adventures-closure-scope-analyzer │  ← this crate
                          └─────────────────────────────────────────┘
                                                   │
                                                   ▼
                            ┌────────────────────────────────────┐
                            │ coding-adventures-javascript-ast    │
                            └────────────────────────────────────┘
```

The constant-fold, fold-control-flow, and dce passes don't need
scope info — they work on expression / control-flow shape only.  The
five passes drawn under this crate's box DO need it; they're CLOC13.A
through CLOC13.E.

## Usage

```rust
use coding_adventures_closure_scope_analyzer::analyze;
use coding_adventures_javascript_ast::Program;

let program: Program = /* … parser produces this … */;
let analysis = analyze(&program);

// Look up a name from a particular scope.
let resolved = analysis.resolve("x", analysis.scopes[0].kind /* etc */);

// Or scan every reference at once (the rename / treeshake passes
// prefer this).
for reference in &analysis.references {
    if reference.binding.is_none() {
        // It's a free global — don't touch it.
    }
}
```

## Status

`analyze` walks the whole program (v0.22.0). The status notes that used to
live here described v0.1.0 and v0.2.0; `CHANGELOG.md` has the full history.

## Which constructs open a scope

| Construct | Scope kind | Holds |
|---|---|---|
| the program | `Global` | top-level `var`, `let`, `const`, functions |
| a function body | `Function` | params, `var` (hoisted), its own `let`/`const` |
| a `{ … }` block | `Block` | `let`, `const`, class and function declarations |
| `for`/`for-in`/`for-of` with a `let`/`const` head | `Block` | the head binding |
| a `switch` case block (all cases share it) | `Block` | `let`/`const` declared in any case |

The last row is why

```js
var x = 1;
for (let x of [7]) console.log(x);
console.log(x);
```

keeps `var x = 1`: the read after the loop resolves past the loop scope to
the outer binding (#16008). A `var` head opens no scope; it hoists like any
other `var`.

## Why a separate crate

See the rationale section in `CHANGELOG.md`.

## License

Apache-2.0.
