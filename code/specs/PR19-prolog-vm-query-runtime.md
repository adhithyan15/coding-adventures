# PR19: Prolog VM Query Runtime

## Summary

This batch adds a stateful query runtime on top of the Prolog VM compiler.

The previous layer proved that source-embedded `?-` queries could run through
the Logic VM. This layer makes the VM path usable in the way Prolog users
expect: consult/load a program once, run initialization once, and issue many
top-level query strings against the same runtime.

## Goals

- provide a long-lived runtime object for compiled Prolog VM programs
- parse ad-hoc SWI query strings through the existing dialect parser
- return named Python answer bindings keyed by source variable name
- support raw value results for callers that prefer the older tuple/singleton
  style
- run compiled initialization slots once before interactive queries
- allow optional query commits so dynamic database effects such as
  `assertz/1` can persist into later queries
- keep direct compiled-program helpers unchanged

## Public API

`prolog-vm-compiler` now exposes:

```python
PrologVMRuntime
create_prolog_vm_runtime(...)
create_swi_prolog_vm_runtime(...)
```

Example:

```python
from prolog_vm_compiler import create_swi_prolog_vm_runtime

runtime = create_swi_prolog_vm_runtime(
    """
    :- dynamic(memo/1).
    parent(homer, bart).
    parent(homer, lisa).
    """,
)

answers = runtime.query("parent(homer, Who)")
runtime.query("assertz(memo(saved))", commit=True)
memo_answers = runtime.query("memo(Value)")
```

## Runtime Behavior

`create_swi_prolog_vm_runtime(...)`:

1. parses and loads one SWI-compatible source string
2. compiles it into `logic-instructions`
3. loads those instructions into `logic-vm`
4. optionally runs initialization query slots
5. returns a reusable `PrologVMRuntime`

`PrologVMRuntime.query(...)` accepts either:

- a full top-level query, such as `"?- parent(homer, Who)."`
- a bare query body, such as `"parent(homer, Who)"`
- an existing `ParsedQuery`

By default, query effects are not committed to the runtime state. Passing
`commit=True` commits persistent stateful overlays such as dynamic database
changes from the first proof state while dropping one-off query substitutions.

## Non-goals

- full Prolog REPL command handling
- project-level ad-hoc module import resolution beyond already compiled
  runtime state
- textual answer formatting
- exact ISO top-level side-effect semantics across all builtins
