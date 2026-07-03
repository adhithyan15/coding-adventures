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

- **v0.1.0 (this release):** types + entry function + serde + tests.
  The body of `analyze` returns a single global scope and no bindings
  — i.e., it doesn't walk the AST yet.  The contract is the unblocker
  for the five consumer passes.
- **v0.2.0 (CLOC13.0, planned):** real body — walks the Program,
  builds the scope tree, populates bindings and references.

## Why a separate crate

See the rationale section in `CHANGELOG.md`.

## License

Apache-2.0.
