# logic-bytecode-vm

`logic-bytecode-vm` executes the LP09 loader-bytecode format directly.

This package sits one layer below `logic-vm`. Instead of stepping rich
`logic-instructions` objects, it walks raw opcode/operand pairs from
`logic-bytecode`, resolves pool references, builds runtime state, and then runs
stored queries through `logic-engine`.

## Dependencies

- logic-engine
- logic-instructions
- logic-bytecode

## Quick Start

```python
from logic_engine import conj, relation, var
from logic_instructions import defrel, fact, instruction_program, query, rule
from logic_bytecode import compile_program
from logic_bytecode_vm import create_logic_bytecode_vm

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

x = var("X")
y = var("Y")
z = var("Z")
who = var("Who")

bytecode = compile_program(
    instruction_program(
        defrel(parent),
        defrel(ancestor),
        fact(parent("homer", "bart")),
        fact(parent("homer", "lisa")),
        rule(ancestor(x, y), parent(x, y)),
        rule(ancestor(x, y), conj(parent(x, z), ancestor(z, y))),
        query(ancestor("homer", who), outputs=(who,)),
    ),
)

vm = create_logic_bytecode_vm()
vm.load(bytecode)
trace = vm.run()
answers = vm.run_query()

assert len(trace) == 8
assert [str(answer) for answer in answers] == ["bart", "lisa"]
```

## Development

```bash
bash BUILD
```
