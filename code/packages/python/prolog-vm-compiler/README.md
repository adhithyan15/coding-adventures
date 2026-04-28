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
