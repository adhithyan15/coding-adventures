# PR18: Prolog VM End-to-End Stress Tests

## Summary

This batch turns the Prolog-on-Logic-VM path into a broader end-to-end surface.

The previous compiler bridge proved that loaded Prolog artifacts could be
lowered into `logic-instructions` and executed by `logic-vm`. PR18 adds stress
coverage and fills the runtime seams exposed by those tests.

## Goals

- prove real Prolog source strings can run through parser, loader, compiler,
  instructions, VM, and engine without switching to the direct loader path
- cover recursive search with structured list answers
- cover linked modules and imported predicates inside meta-predicate callables
- cover DCG expansion through `phrase/3`
- cover arithmetic evaluation and comparison through the Prolog builtin adapter
- cover collection predicates such as `findall/3`
- cover dynamic database startup through compiled initialization queries
- expose named Python answer bindings for source-level query variables
- preserve source query variables across goal expansion rewrites

## Runtime Additions

`logic-vm` now supports query execution from an existing `State`:

```python
vm.solve_query_from(state, query_index=0)
vm.run_query_from(state, query_index=0, limit=None)
vm.run_all_queries_from(state, limit=None)
```

This keeps the default `run_query(...)` behavior simple, while giving higher
layers a way to run initialization goals first and then execute later source
queries against the initialized dynamic database.

## Compiler Additions

`prolog-vm-compiler` now exposes named and initialized helpers:

```python
run_compiled_prolog_query_answers(...)
run_compiled_prolog_initializations(...)
run_initialized_compiled_prolog_query(...)
run_initialized_compiled_prolog_query_answers(...)
```

`PrologAnswer` is a lightweight immutable answer object with `as_dict()` for
Python-facing use.

## Loader And Adapter Additions

The loader now preserves query variable identities when goal expansion rewrites
a query. This matters for source-level query answers: the VM must reify the
variables that actually occur in the expanded executable goal, not stale
pre-expansion variable objects.

Module rewriting now reaches callable arguments for meta-predicates:

- `findall/3`
- `bagof/3`
- `setof/3`
- `forall/2`

The shared Prolog builtin adapter now lowers more common Prolog builtins:

- `true/0`, `fail/0`, `!/0`
- `is/2`
- `=:=/2`, `=\=/2`, `</2`, `=</2`, `>/2`, `>=/2`
- `findall/3`, `bagof/3`, `setof/3`
- `forall/2`
- `copy_term/2`

## Stress Test Scenarios

The new stress suite verifies:

- recursive path enumeration returns list-valued answers
- modules, DCGs, arithmetic, and `findall/3` compose in one VM query
- initialization directives can declare and assert dynamic facts before a
  source query runs
- named answer helpers return dictionaries keyed by source variable names
- loader `term_expansion/2` and `goal_expansion/2` are preserved before VM
  compilation

## Remaining Gaps

This does not claim full Prolog parity yet. Remaining important gaps include:

- full ISO/SWI builtin coverage
- richer error behavior for instantiation/type/domain errors
- lower-level bytecode parity for every high-level instruction
- indexing/performance work for larger programs
- full module-aware treatment for every possible meta-predicate shape
