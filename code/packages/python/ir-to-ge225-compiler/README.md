# ir-to-ge225-compiler

GE-225 backend: lowers a target-independent `IrProgram` to GE-225 20-bit machine words.

## Overview

This package is the machine-code backend in the Dartmouth BASIC compiled pipeline:

```
BASIC source
    ↓  dartmouth-basic-lexer     (tokenize)
    ↓  dartmouth-basic-parser    (parse to AST)
    ↓  dartmouth-basic-ir-compiler   (lower to IR)
    ↓  ir-to-ge225-compiler          (this package — emit GE-225 words)
    ↓  ge225-simulator               (execute)
```

It accepts a `compiler_ir.IrProgram` and emits a binary blob of GE-225 20-bit
words (packed as 3 bytes per word, big-endian) ready to load into `GE225Simulator`.

## Usage

```python
from compiler_ir import IrProgram, IrInstruction, IrOp, IrRegister, IrImmediate, IrLabel
from ir_to_ge225_compiler import compile_to_ge225
from ge225_simulator import GE225Simulator, unpack_words

# Build a simple program: v1 = 3 + 4; HALT
program = IrProgram(entry_label="_start")
# ... populate program.instructions ...

result = compile_to_ge225(program)

sim = GE225Simulator(memory_words=4096)
sim.load_program_bytes(result.binary)

# Run until halt
while True:
    trace = sim.step()
    if trace.address == result.halt_address:
        break

# Read variable A (v1) from its spill slot
a_value = sim.read_word(result.data_base + 1)
```

## Memory Layout

```
┌─────────────────────────────────────┐
│  addr 0       : TON (prologue)      │
│  addr 1…      : compiled IR code    │
│  addr code_end: BRU code_end (halt) │
│  addr data_base…: spill slots (v0…) │
│  addr …       : constants table     │
└─────────────────────────────────────┘
```

- `spill_addr(N) = data_base + N`
- `const_addr(K) = data_base + n_regs + K`

## V1 Supported IR Opcodes

| IR Opcode | GE-225 Words |
|-----------|-------------|
| LABEL / COMMENT | 0 |
| NOP | 1 |
| HALT | 1 |
| JUMP | 1 |
| LOAD_IMM | 2 |
| ADD / SUB | 3 |
| ADD_IMM (copy) | 2 |
| ADD_IMM (±1) | 3 |
| ADD_IMM (other) | 3 |
| AND_IMM (imm=1) | 7 |
| MUL | 6 |
| DIV | 5 |
| CMP_EQ / CMP_NE / CMP_LT / CMP_GT | 8 |
| BRANCH_Z / BRANCH_NZ | 3 |
| SYSCALL 1 | 3 |

## Historical Note

The GE-225 was a 20-bit word-addressed accumulator machine built by General Electric
in 1960. Dartmouth's time-sharing BASIC system, designed in 1964 by John Kemeny and
Thomas Kurtz, ran on this hardware. All arithmetic flows through the single accumulator
register A; every variable is stored in a spill slot (a dedicated memory word). This
backend faithfully mirrors that architecture: no register allocation, pure spill-based
code generation.
