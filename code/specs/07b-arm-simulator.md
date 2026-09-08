# 04 — ARM Simulator

## Overview

The ARM simulator implements a subset of the ARM instruction set architecture (ISA). It decodes real ARM binary instructions and executes them on the CPU simulator. This is where abstract computation meets a real-world processor design.

We implement ARMv7 (32-bit) because it is simpler than ARMv8/AArch64 (64-bit) while still being a real, widely-used architecture (Raspberry Pi, older phones, embedded systems).

This is Layer 4 of the computing stack. It depends on the CPU simulator.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

**Input from:** CPU simulator (registers, memory, ALU, fetch-decode-execute cycle).
**Output to:** Assembler (defines the binary encoding that the assembler produces).

## Concepts

### ARM Instruction Encoding

Every ARM instruction is exactly 32 bits (4 bytes). The bits are divided into fields:

```
31  28 27 26 25 24  21 20 19  16 15  12 11           0
┌─────┬─────┬──┬──────┬──┬──────┬──────┬─────────────┐
│Cond │ 00  │I │Opcode│S │  Rn  │  Rd  │  Operand2   │
└─────┴─────┴──┴──────┴──┴──────┴──────┴─────────────┘
```

- **Cond (4 bits)**: Condition code — execute only if flags match (EQ, NE, GT, LT, AL=always)
- **I (1 bit)**: Is operand2 an immediate value or a register?
- **Opcode (4 bits)**: Which operation (ADD, SUB, MOV, CMP, etc.)
- **S (1 bit)**: Should this instruction update the flags?
- **Rn (4 bits)**: First operand register
- **Rd (4 bits)**: Destination register
- **Operand2 (12 bits)**: Second operand (register or immediate)

### Instruction Subset (MVP)

Starting with the minimum needed to run `x = 1 + 2`:

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| MOV      | 1101   | Move value into register |
| ADD      | 0100   | Add two values |
| SUB      | 0010   | Subtract |
| CMP      | 1010   | Compare (sets flags, no result stored) |
| AND      | 0000   | Bitwise AND |
| ORR      | 1100   | Bitwise OR |
| LDR      | —      | Load from memory into register |
| STR      | —      | Store register value to memory |
| B        | —      | Branch (jump to address) |
| BEQ/BNE  | —      | Conditional branch |
| HLT      | —      | Halt execution |

### Condition Codes

ARM's signature feature: almost every instruction can be conditionally executed based on flags.

| Code | Suffix | Condition |
|------|--------|-----------|
| 0000 | EQ     | Zero flag set (equal) |
| 0001 | NE     | Zero flag clear (not equal) |
| 1010 | GE     | Greater or equal (signed) |
| 1011 | LT     | Less than (signed) |
| 1100 | GT     | Greater than (signed) |
| 1101 | LE     | Less or equal (signed) |
| 1110 | AL     | Always (unconditional) |

## Public API

```python
class ARMSimulator:
    def __init__(self, memory_size: int = 65536) -> None: ...
        # Creates a CPU with 16 registers (R0-R15), R15 = PC, R14 = LR, R13 = SP

    @property
    def registers(self) -> list[int]: ...

    @property
    def memory(self) -> bytearray: ...

    @property
    def flags(self) -> Flags: ...

    def load_program(self, machine_code: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Instruction: ...
        # Execute one instruction, return the decoded instruction for inspection

    def run(self, max_steps: int = 10000) -> list[Instruction]: ...
        # Run until halted, return trace of executed instructions

@dataclass
class Instruction:
    address: int            # Where in memory this instruction was
    raw: int                # The 32-bit binary instruction
    mnemonic: str           # Human-readable name (e.g., "ADD")
    condition: str          # Condition code (e.g., "AL")
    rd: int | None          # Destination register
    rn: int | None          # First operand register
    operand2: int | None    # Second operand (register index or immediate value)
    update_flags: bool      # Does this instruction update flags?

def decode(instruction: int) -> Instruction: ...
    # Decode a 32-bit integer into an Instruction
```

### Rust completion contract

The Rust package preserves the historical `ARMSimulator` wrapper and exposes
the normative checked machine as `functional::Armv7Simulator`. Its complete
state is exactly 64 KiB of little-endian memory, R0–R15, a mirrored PC/R15,
NZCV, halt, and the installed origin/length. Restore validates the whole state;
loading is origin-aware and checked; direct access is typed; a failing step
commits nothing; and a bounded run either returns complete before/after traces
or restores its entry state.

The completed Rust decode surface is MOV, ADD, SUB, CMP, AND, ORR, word
LDR/STR with pre-indexed immediate offsets, B/BL (using ARM's PC+8 base), all
sixteen condition predicates, and the custom HLT sentinel. Unsupported operand2
shifts and addressing modes fail closed rather than being mis-executed. The
documented S bit updates NZCV; CMP always updates NZCV and never writes Rd.

Verification includes strict formatting, Clippy and rustdoc, lifecycle and
manual Spec 07b suites, and a reproducible 388-vector Python common-surface
full-state corpus. Rust line coverage is 90.15% (714/792 package lines).

## Data Flow

```
Input:  Raw bytes (machine code) loaded into memory
Output: Instruction trace + final CPU state (registers, memory, flags)
```

## Test Strategy

- Decode known ARM instruction encodings and verify fields
- Execute MOV: verify register receives correct value
- Execute ADD: verify R2 = R0 + R1
- Execute SUB: verify subtraction and flag setting
- Execute CMP: verify only flags change, no register write
- Execute conditional instructions: verify they skip when condition is false
- Execute LDR/STR: verify memory load/store
- Execute B (branch): verify PC changes
- End-to-end: run the `x = 1 + 2` program (MOV R0, #1; MOV R1, #2; ADD R2, R0, R1) and verify R2 = 3

## Future Extensions

- **More instructions**: MUL, shift operations, stack push/pop
- **Thumb mode**: 16-bit compressed instruction set
- **System calls**: SVC instruction for simulating OS interactions
- **Floating point**: VFP/NEON instruction simulation
