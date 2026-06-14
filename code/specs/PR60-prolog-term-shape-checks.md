# PR60: Prolog Term Shape Checks

## Goal

Expose standard Prolog finite-term shape predicates in the library and VM
path. This lets source-level programs ask whether a term is acyclic or cyclic
without dropping down into implementation-specific term inspection.

## Scope

The builtin and loader layers expose:

```text
acyclic_term/1
cyclic_term/1
```

The VM path supports both through parser, loader adapter, compiler, and runtime
execution.

## Semantics

This batch implements the predicates against the current `logic-engine` term
model:

- `acyclic_term(Term)` succeeds when the reified term graph has no recursive
  compound path.
- Variables, atoms, numbers, strings, and ordinary finite compounds are
  acyclic.
- `cyclic_term(Term)` succeeds when the reified term graph contains a cycle.
- Today, user-created source terms cannot form rational trees because the core
  term model is immutable and finite. As a result, `cyclic_term/1` normally
  fails, but the predicate and traversal are in place for future cyclic-term
  support.

## Verification

- `logic-builtins` tests cover finite/open acyclic terms and current
  `cyclic_termo/1` failure behavior.
- `prolog-loader` tests cover source-level adaptation for both predicates.
- `prolog-vm-compiler` stress coverage runs both predicates end-to-end through
  parser, loader, compiler, VM, and named-answer extraction.
