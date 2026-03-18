# 03 — CPU Simulator

## Overview

The CPU simulator models the core components of a processor: registers (fast storage), memory (large storage), a program counter (which instruction to execute next), and the fetch-decode-execute cycle that drives all computation.

This is a generic CPU model, not tied to any specific architecture. The ARM simulator (Layer 4) will build on this to implement a real instruction set.

This is Layer 3 of the computing stack. It depends on the arithmetic package.

## Layer Position

```
Logic Gates → Arithmetic → [YOU ARE HERE] → ARM → Assembler → Lexer → Parser → Compiler → VM
```

**Input from:** Arithmetic (provides the ALU for computation).
**Output to:** ARM simulator (extends this with a real instruction set).

## Concepts

### Registers

Registers are small, fast storage locations inside the CPU. A typical CPU has 8-32 registers, each holding one word (e.g., 32 bits). Operations happen between registers — you load data from memory into a register, operate on it, then store it back.

### Memory

Memory is a large array of bytes. The CPU accesses it via addresses (indices). Memory is much slower than registers but much larger.

### Program Counter (PC)

The PC is a special register that holds the address of the next instruction to execute. After each instruction, the PC advances. Branch/jump instructions change the PC to a different address, enabling loops and conditionals.

### Fetch-Decode-Execute Cycle

This is the heartbeat of every CPU:

```
1. FETCH:   Read the instruction at the address in the PC
2. DECODE:  Figure out what the instruction means (what operation, which registers)
3. EXECUTE: Perform the operation using the ALU
4. STORE:   Write the result to a register or memory
5. ADVANCE: Increment the PC (unless a branch changed it)
6. REPEAT
```

### Status Flags

The CPU maintains flags that reflect the result of the last ALU operation:
- Zero, Carry, Negative, Overflow (from the ALU)

These flags are used by conditional branch instructions (e.g., "branch if zero").

## Public API

```python
@dataclass
class CPUState:
    registers: list[int]       # General purpose registers
    memory: bytearray          # Main memory
    pc: int                    # Program counter
    flags: Flags               # Status flags
    halted: bool               # Has the CPU stopped?

@dataclass
class Flags:
    zero: bool
    carry: bool
    negative: bool
    overflow: bool

class CPU:
    def __init__(
        self,
        num_registers: int = 16,
        memory_size: int = 65536,
        bit_width: int = 32,
    ) -> None: ...

    @property
    def state(self) -> CPUState: ...

    def load_program(self, program: bytes, start_address: int = 0) -> None: ...
        # Load machine code into memory

    def step(self) -> None: ...
        # Execute one fetch-decode-execute cycle

    def run(self, max_steps: int = 10000) -> None: ...
        # Run until halted or max_steps reached

    def read_register(self, index: int) -> int: ...
    def write_register(self, index: int, value: int) -> None: ...
    def read_memory(self, address: int, num_bytes: int = 1) -> bytes: ...
    def write_memory(self, address: int, data: bytes) -> None: ...
```

## Data Flow

```
Input:  A program (bytes) loaded into memory
Output: Modified register and memory state after execution
```

The CPU is a state machine. Each `step()` call transitions from one state to the next based on the current instruction.

## Test Strategy

- Verify initial state (all registers zero, PC at 0, not halted)
- Verify memory load and read/write
- Verify register read/write
- Verify PC advances after each step
- Verify halting condition
- Verify flags are set correctly after ALU operations
- Verify `run()` stops at max_steps
- Test with a simple hand-crafted program (load, add, store, halt)

## Future Extensions

- **Interrupts**: External signals that pause normal execution
- **Pipeline simulation**: Model instruction pipelining (fetch one instruction while executing another)
- **Cache simulation**: Model L1/L2 cache behavior to understand memory hierarchy
- **I/O ports**: Memory-mapped I/O for interacting with simulated peripherals
