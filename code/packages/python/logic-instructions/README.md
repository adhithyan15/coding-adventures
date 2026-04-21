# logic-instructions

`logic-instructions` standardizes logic programs as an ordered stream of
instructions.

It sits between the direct Python API in [`logic-engine`](../logic-engine/) and
the future VM backend we want to build next. The first version does not expose
low-level bytecode yet. Instead, it turns:

- relation declarations
- dynamic relation declarations
- facts
- rules
- queries

into explicit executable data that can already be validated and run through the
current engine backend.

## Instruction Set

- `DEF_REL`
- `DYNAMIC_REL`
- `FACT`
- `RULE`
- `QUERY`

## Quick Start

```python
from logic_engine import atom, conj, relation, var
from logic_instructions import defdynamic, defrel, fact, instruction_program, query, rule, run_query

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

program_ops = instruction_program(
    defrel(parent),
    defdynamic(ancestor),
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

assert run_query(program_ops) == [atom("bart"), atom("lisa")]
```

## Why This Package Exists

This instruction stream becomes a shared contract between:

- the current direct engine
- future compiler passes
- future Prolog lowering
- future logic VM execution

## Dependencies

- [`logic-engine`](../logic-engine/)
- [`symbol-core`](../symbol-core/)

## Development

```bash
bash BUILD
```
