# logic-vm

`logic-vm` is the first executable runtime for the LP07 instruction stream.

It does not introduce lower-level bytecode yet. Instead, it executes the
existing `DEF_REL`, `DYNAMIC_REL`, `FACT`, `RULE`, and `QUERY` instructions through a small
dispatch-table VM, accumulates runtime state incrementally, and then runs
stored queries with the current `logic-engine` backend.

## What It Adds

- a mutable runtime state for instruction loading
- single-step execution with trace entries
- dispatch handlers keyed by `InstructionOpcode`
- query execution after program loading completes

## Dependencies

- logic-instructions
- logic-engine

## Quick Start

```python
from logic_engine import atom, conj, relation, var
from logic_instructions import defdynamic, defrel, fact, instruction_program, query, rule
from logic_vm import create_logic_vm

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

program_value = instruction_program(
    defrel(parent),
    defdynamic(ancestor),
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

vm = create_logic_vm()
vm.load(program_value)
trace = vm.run()

assert trace[-1].query_count == 1
assert vm.run_query() == [atom("bart"), atom("lisa")]
assert vm.assembled_program().dynamic_relations == frozenset({ancestor.key()})
```

## Development

```bash
bash BUILD
```
