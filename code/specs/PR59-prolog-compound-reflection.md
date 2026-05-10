# PR59: Prolog Compound Reflection

## Goal

Add compound-only term reflection predicates to the library, loader, and VM
path. This fills the gap between broad `functor/3` or `=../2` term
metaprogramming and modern Prolog code that wants to inspect or construct only
compound terms.

## Scope

The builtin and loader layers expose:

```text
compound_name_arguments/3
compound_name_arity/3
```

The VM path supports both through parser, loader adapter, compiler, and runtime
execution.

## Semantics

This batch implements finite, compound-only reflection:

- `compound_name_arguments(Compound, Name, Arguments)` decomposes a compound
  into its atom functor name and proper list of arguments.
- When `Compound` is open, a bound atom `Name` and non-empty proper
  `Arguments` list construct a compound.
- Atomic terms fail cleanly because these predicates are intentionally
  compound-only.
- `compound_name_arity(Compound, Name, Arity)` decomposes a compound into its
  atom functor name and integer arity.
- When `Compound` is open, a bound atom `Name` and positive integer `Arity`
  construct a compound with fresh argument variables.
- Zero-arity construction is rejected here because it would create an atom,
  not a compound. Callers that want atomic zero-arity behavior can continue to
  use `functor/3`.

## Verification

- `logic-builtins` tests cover inspection, construction, atomic rejection,
  invalid argument lists, invalid names, and invalid arities.
- `prolog-loader` tests cover source-level adaptation for both predicates.
- `prolog-vm-compiler` stress coverage runs both forms end-to-end through
  parser, loader, compiler, VM, and named-answer extraction.
