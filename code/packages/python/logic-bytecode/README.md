# logic-bytecode

`logic-bytecode` compiles LP07 instruction streams into a compact loader
bytecode format.

This is not a full WAM or proof-search bytecode yet. It is the first compact
opcode layer between rich `logic-instructions` objects and future lower-level
VM execution.

## What It Adds

- integer bytecode opcodes
- index-based operands and pools
- compile and decode helpers
- readable disassembly for debugging and tests

## Dependencies

- logic-instructions
- logic-engine

## Quick Start

```python
from logic_engine import relation, var
from logic_instructions import defrel, fact, instruction_program, query
from logic_bytecode import compile_program, decode_program, disassemble_text

parent = relation("parent", 2)
who = var("Who")

program_value = instruction_program(
    defrel(parent),
    fact(parent("homer", "bart")),
    query(parent("homer", who)),
)

bytecode = compile_program(program_value)
round_tripped = decode_program(bytecode)

assert round_tripped == program_value
print(disassemble_text(bytecode))
```

## Development

```bash
bash BUILD
```
