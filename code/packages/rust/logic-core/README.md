# logic-core (Rust)

Terms, logic variables, substitutions, and unification — the semantic core of
a logic VM.

## What This Is

`logic-core` is the data layer of a logic programming engine. It provides:

- a small term language (atoms, numbers, strings, variables, compound terms),
- logic variables with stable identity,
- copy-on-write substitutions (variable-to-term bindings), and
- first-order **unification** with occurs-check.

It does not yet contain goals, search, disequality, or a Prolog parser. Those
arrive in follow-up crates (`logic-engine`, `logic-builtins`, `prolog-core`,
...), mirroring the Python package family already in this repo.

## How It Fits in the Stack

This is a Rust port of `code/packages/python/logic-core`, which implements the
language-agnostic specification in [`code/specs/LP00-logic-core.md`](../../../specs/LP00-logic-core.md).

```
   spec  LP00-logic-core.md             ← language-agnostic spec
                 │
                 ├── python/logic-core   ← reference implementation
                 └── rust/logic-core     ← this crate
```

Subsequent Rust crates will port the rest of the LP and PR specs (logic
engine, builtins, bytecode, Prolog frontend) one layer at a time.

## API at a Glance

```rust
use logic_core::{atom, var, compound, unify, Substitution, Term};

// father(homer, bart).
let father_homer_bart = compound("father", vec![atom("homer"), atom("bart")]);

// father(homer, X).
let x = var("X");
let query = compound("father", vec![atom("homer"), Term::Var(x.clone())]);

// Unify the query against the fact.
let subst = unify(&query, &father_homer_bart, &Substitution::empty())
    .expect("unification should succeed");

// X is now bound to `bart`.
assert_eq!(subst.walk_var(&x).to_string(), "bart");
```

## Why a Rust Port

The Python implementation is the reference; the Rust port adds:

- **Predictable performance** with no GC pauses, suitable for embedding in
  other tools (LSPs, IDE extensions, batch verifiers).
- **A statically-typed term language** that makes pattern-matching on terms
  in downstream crates a compile-time safety property.
- **A small dependency footprint** — at the time of writing, `logic-core` has
  no external crates.

## Status

Experimental. The API will evolve as the rest of the Rust port lands. Pin to
a specific commit if you depend on it from outside this workspace.
