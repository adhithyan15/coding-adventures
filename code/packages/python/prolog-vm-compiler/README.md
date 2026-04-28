# prolog-vm-compiler

`prolog-vm-compiler` lowers loaded Prolog artifacts into standardized
`logic-instructions` programs that can run through `logic-vm`.

It is the bridge between the Prolog frontend stack and the Logic VM:

- parse and load source with `prolog-loader`
- collect relation declarations, dynamic declarations, clauses, and queries
- compile ground Prolog facts to `FACT`
- compile Prolog rules and variable-bearing facts to `RULE`
- preserve source queries as VM `QUERY` instructions
- optionally adapt supported Prolog builtins into executable logic builtin goals
- run source queries with named answer bindings
- run initialization query slots before later source queries when callers want
  stateful dynamic startup behavior

## Quick Start

```python
from logic_engine import atom
from prolog_vm_compiler import compile_swi_prolog_source, run_compiled_prolog_query

compiled = compile_swi_prolog_source(
    """
    parent(homer, bart).
    parent(homer, lisa).
    ancestor(X, Y) :- parent(X, Y).
    ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

    ?- ancestor(homer, Who).
    """,
)

assert run_compiled_prolog_query(compiled) == [atom("bart"), atom("lisa")]
```

## Named Answers And Initialization

```python
from logic_engine import atom
from prolog_vm_compiler import (
    compile_swi_prolog_source,
    run_initialized_compiled_prolog_query_answers,
)

compiled = compile_swi_prolog_source(
    """
    :- initialization(dynamic(seen/1)).
    :- initialization(assertz(seen(alpha))).

    ?- seen(Name).
    """,
)

answers = run_initialized_compiled_prolog_query_answers(compiled)

assert [answer.as_dict() for answer in answers] == [{"Name": atom("alpha")}]
```

## Stateful Query Runtime

Use `create_swi_prolog_vm_runtime(...)` when you want to consult a program once
and ask many top-level queries later:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_vm_runtime

runtime = create_swi_prolog_vm_runtime(
    """
    :- dynamic(memo/1).
    parent(homer, bart).
    parent(homer, lisa).
    """,
)

assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]

runtime.query("assertz(memo(saved))", commit=True)

assert [answer.as_dict() for answer in runtime.query("memo(Value)")] == [
    {"Value": atom("saved")},
]
```

## File Runtime

Use `create_swi_prolog_file_runtime(...)` to load a `.pl` file once and query it
through the same stateful VM runtime:

```python
from logic_engine import atom
from prolog_vm_compiler import create_swi_prolog_file_runtime

runtime = create_swi_prolog_file_runtime("app.pl")

assert [answer.as_dict() for answer in runtime.query("ancestor(homer, Who)")] == [
    {"Who": atom("bart")},
    {"Who": atom("lisa")},
]
```

For linked file graphs with source-level `?-` queries, use
`compile_swi_prolog_project_from_files(...)` and the compiled query helpers.

## Stress Coverage

The package includes end-to-end stress tests for:

- recursive graph/path search
- linked modules and imported predicates
- DCG expansion with `phrase/3`
- arithmetic evaluation and comparison
- collection predicates such as `findall/3`
- dynamic predicates seeded by initialization directives
- named Python answer bindings
- loader term/goal expansion before VM compilation

## Layer Position

```text
Prolog dialect parser
    ↓
prolog-loader
    ↓
prolog-vm-compiler
    ↓
logic-instructions
    ↓
logic-vm
```

## Development

```bash
bash BUILD
```
