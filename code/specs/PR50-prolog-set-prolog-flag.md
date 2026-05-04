# PR50 - Branch-Local Prolog Runtime Flags

## Goal

Add a backtracking-safe `set_prolog_flag/2` surface on top of the read-only
`current_prolog_flag/2` support from PR49.

## Motivation

Dialect libraries and portable Prolog programs often configure runtime behavior
with flags before running compatibility code. The Logic VM should support that
style without introducing process-global mutable state. Flag changes therefore
belong in the search `State`, just like dynamic database overlays and finite
domain stores.

## Design

`logic-core.State` gains a `prolog_flags` extension slot. The core and engine
state-copy helpers preserve it across unification, disequality checks, fresh
variable allocation, rule invocation, dynamic database changes, and CLP(FD)
store updates.

`logic-builtins` owns the concrete flag store:

```python
PrologFlagStore(values={...})
```

`current_prolog_flago/2` reads the default flag table plus branch-local
overrides. `set_prolog_flago/2` updates only supported writable flags in the
current proof branch.

## Supported Writable Flags

- `char_conversion`: `false`, `true`
- `debug`: `false`, `true`
- `double_quotes`: `atom`, `chars`, `codes`, `string`
- `occurs_check`: `false`, `true`, `error`
- `unknown`: `error`, `fail`, `warning`

Read-only flags such as `bounded` and `integer_rounding_function` are visible
through `current_prolog_flag/2` but cannot be changed by this first batch.

## Semantics

- Flag updates are visible to later goals in the same proof branch.
- Flag updates are undone when search backtracks to another branch.
- Unknown flag names, read-only flags, and unsupported values fail.
- Uninstantiated flag names or values raise a source-level instantiation error.

## Acceptance Tests

- Core goals preserve the new `State.prolog_flags` slot.
- Engine state-copy helpers preserve the new slot.
- `set_prolog_flago/2` updates `current_prolog_flago/2` in the active branch.
- Updates roll back across disjunction branches.
- Loader adapts parsed `set_prolog_flag/2`.
- VM stress coverage proves source-level `set_prolog_flag/2` and
  `current_prolog_flag/2` compose through the full parser-to-VM path.
