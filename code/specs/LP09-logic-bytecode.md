# LP09 — Logic Bytecode

## Overview

The logic stack now has:

- `logic-instructions` — a standardized *program-level* instruction stream
- `logic-vm` — a dispatch runtime that executes those high-level instructions

That is enough to load and run relational programs, but we still do not have a
compact lower-level representation that looks and feels like real bytecode.

LP09 introduces that next layer.

## Design Goal

Add a Python package that compiles LP07 instruction streams into a compact
bytecode object.

This first bytecode format is deliberately modest. It is not a WAM and it is
not yet the final execution format for unification and backtracking. Instead,
it is a **loader bytecode**:

- compact
- index-based
- deterministic
- easy to serialize later
- easy to disassemble for debugging
- easy to decode back into LP07 instruction streams

## Why A Loader Bytecode First

Jumping directly from LP07 to a full WAM-style execution format would collapse
too many ideas into one milestone.

We first want a bytecode layer that answers:

- what is the compact opcode form of the current instruction stream?
- how do we move from rich Python objects to pool-indexed instructions?
- how do we inspect and round-trip that format?

That gives us a stable "bytecode object" boundary before later milestones add:

- low-level call and choice-point opcodes
- trail management
- backtracking instructions
- clause-level control flow

## Package

Add a new Python package:

```text
code/packages/python/logic-bytecode
```

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    ↓
LP01 Logic Engine
    ↓
LP07 Logic Instructions
    ↓
LP08 Logic VM
    ↓
LP09 Logic Bytecode   ← this milestone
```

## Scope

LP09 should compile the existing LP07 instruction stream into a lower-level
bytecode object with:

- opcode integers
- index operands
- separate pools for referenced objects
- a terminal halt instruction

It should also decode that bytecode back into an `InstructionProgram`.

## Instruction Set

The first bytecode format should define these opcodes:

- `EMIT_RELATION`
- `EMIT_FACT`
- `EMIT_RULE`
- `EMIT_QUERY`
- `HALT`

This is still a loader bytecode, not a proof-search bytecode.

## Bytecode Model

The package should expose:

```python
LogicBytecodeOp
LogicBytecodeInstruction
LogicBytecodeProgram
LogicBytecodeDisassemblyLine
LogicBytecodeError
compile_program
decode_program
disassemble
disassemble_text
```

### Opcode enum

Use an `IntEnum` so opcodes are raw integer byte values but still print
human-readable names.

Suggested layout:

```python
class LogicBytecodeOp(IntEnum):
    EMIT_RELATION = 0x00
    EMIT_FACT = 0x01
    EMIT_RULE = 0x02
    EMIT_QUERY = 0x03
    HALT = 0xF0
```

### Instruction object

```python
@dataclass(frozen=True, slots=True)
class LogicBytecodeInstruction:
    opcode: int
    operand: int | None = None
```

Why `int` instead of only `LogicBytecodeOp`?

Because bytecode is fundamentally raw numeric data, and using `int` makes it
possible to test malformed or unknown opcode values directly.

### Bytecode program

```python
@dataclass(frozen=True, slots=True)
class LogicBytecodeProgram:
    instructions: tuple[LogicBytecodeInstruction, ...]
    relation_pool: tuple[Relation, ...]
    fact_pool: tuple[FactInstruction, ...]
    rule_pool: tuple[RuleInstruction, ...]
    query_pool: tuple[QueryInstruction, ...]
```

## Compilation Rules

`compile_program(...)` should:

1. Validate the incoming LP07 instruction stream first.
2. Create pool entries for referenced relations, facts, rules, and queries.
3. Emit one bytecode instruction per LP07 instruction.
4. Append `HALT` as the final instruction.

### Pooling rules

- relation declarations should use a deduplicated relation pool keyed by
  `(symbol, arity)`
- facts, rules, and queries should preserve source multiplicity
- compilation must preserve source order in the instruction stream

## Decoding Rules

`decode_program(...)` should:

- walk the bytecode instruction list
- reconstruct the original LP07 instructions from the operand pools
- stop on `HALT`
- reject malformed programs:
  - unknown opcodes
  - missing operands for pool-indexed instructions
  - out-of-range pool indexes

The decoded instruction stream should round-trip cleanly with the compiler.

## Disassembly

Bytecode should be easy to inspect in tests, logs, and later tooling.

Expose:

```python
def disassemble(program: LogicBytecodeProgram) -> tuple[LogicBytecodeDisassemblyLine, ...]
def disassemble_text(program: LogicBytecodeProgram) -> str
```

Each disassembly line should include:

- instruction index
- opcode name
- operand
- a short human-readable rendering of the referenced pool entry when available

Example:

```text
0000: EMIT_RELATION 0 ; parent/2
0001: EMIT_FACT 0 ; parent(homer, bart)
0002: EMIT_QUERY 0 ; parent(homer, Who)
0003: HALT
```

## Error Model

Use explicit bytecode-specific errors for:

- unknown opcode values
- missing operands
- invalid pool indexes
- malformed bytecode streams

## Usage Example

```python
from logic_engine import conj, relation, var
from logic_instructions import defrel, fact, instruction_program, query, rule
from logic_bytecode import compile_program, decode_program, disassemble_text

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

program_value = instruction_program(
    defrel(parent),
    defrel(ancestor),
    fact(parent("homer", "bart")),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

bytecode = compile_program(program_value)
round_tripped = decode_program(bytecode)

assert round_tripped == program_value
print(disassemble_text(bytecode))
```

## Test Strategy

Required tests:

- compile a real LP07 instruction stream and verify opcode sequence
- verify `HALT` is appended
- verify relation declarations share a deduplicated relation pool
- round-trip compile → decode → original instruction program
- disassemble a small bytecode program to readable text
- reject unknown opcode values
- reject missing operands
- reject out-of-range pool indexes

## Future Extensions

LP09 prepares the logic stack for the next real VM transition.

Later milestones may add:

- a bytecode-consuming VM path in `logic-vm`
- lower-level execution opcodes
- labels and jump targets
- clause-entry metadata
- a WAM-inspired backend
- binary serialization of logic bytecode

## Why This Milestone Matters

LP07 gave us a standardized logic program representation.

LP08 gave us a runtime for that representation.

LP09 gives us the first compact opcode form in between them.

That is the missing bridge between:

- rich Python instruction objects
- future bytecode execution
- debugging and disassembly
- eventual serialization and alternate runtimes
