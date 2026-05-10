# PR74: Prolog Source Runner API

## Goal

Make the Prolog library path pleasant for one-shot Python use.

Earlier PRs made the Prolog frontend compile to both structured and bytecode VM
backends. This PR adds source-level helpers so callers can go directly from a
Prolog source string to answers without manually compiling first.

## Scope

- Add `run_prolog_source_query(...)` for raw tuple/singleton values.
- Add `run_prolog_source_query_answers(...)` for named `PrologAnswer` values.
- Add SWI and ISO convenience wrappers.
- Preserve backend selection through the same `backend="structured"` and
  `backend="bytecode"` option.
- Support optional initialization execution before the selected source query.

## Examples

```python
from logic_engine import atom
from prolog_vm_compiler import run_swi_prolog_source_query

answers = run_swi_prolog_source_query(
    """
    parent(homer, bart).
    parent(homer, lisa).
    ?- parent(homer, Who).
    """,
    backend="bytecode",
)

assert answers == [atom("bart"), atom("lisa")]
```

## Acceptance

- One-shot source helpers produce the same answers as compile-then-run helpers.
- Named answer helpers preserve source variable names.
- Initialization directives can seed dynamic state before a one-shot query.
- ISO and SWI convenience wrappers route through the correct dialect profiles.

## Future Direction

File and project one-shot runners can follow the same shape if callers want a
non-stateful API for consulted file graphs. Stateful runtime helpers remain the
right abstraction for repeated top-level queries.
