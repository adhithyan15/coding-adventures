# 05 — Assembler

## Overview

The assembler translates human-readable assembly language into binary machine code. It bridges the gap between text that humans can write and read ("MOV R0, #1") and the raw bytes that the CPU executes (0xE3A00001).

This is Layer 5 of the computing stack. It connects the software layers above with the hardware layers below.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → [YOU ARE HERE] → Lexer → Parser → Compiler → VM
```

**Input from:** ARM simulator (defines the binary instruction encoding format).
**Output to:** The compiler can target the assembler to produce machine code (Path B: compiled execution).

## Concepts

### Assembly Language

Assembly is a thin textual representation of machine instructions. Each line typically maps to exactly one machine instruction:

```asm
MOV R0, #1          ; Load 1 into register 0
MOV R1, #2          ; Load 2 into register 1
ADD R2, R0, R1      ; R2 = R0 + R1
HLT                 ; Stop execution
```

### Labels

Labels name memory locations so you can branch to them without hardcoding addresses:

```asm
start:
    MOV R0, #10
    CMP R0, #0
    BEQ done          ; Branch to 'done' if R0 == 0
    SUB R0, R0, #1
    B start            ; Branch back to 'start'
done:
    HLT
```

### Two-Pass Assembly

The assembler runs two passes over the source:

1. **Pass 1**: Scan for labels, record their addresses in a symbol table
2. **Pass 2**: Encode each instruction into binary, resolving label references using the symbol table

This is necessary because a branch instruction might reference a label that appears later in the file (a "forward reference").

### Directives

Directives are instructions to the assembler, not to the CPU:

```asm
.data                ; Switch to data section
message: .asciz "Hello"  ; Store a string in memory
.text                ; Switch to code section
.global _start       ; Mark entry point
```

## Public API

```python
class Assembler:
    def __init__(self) -> None: ...

    def assemble(self, source: str) -> AssemblyResult: ...
        # Assemble source text into machine code

@dataclass
class AssemblyResult:
    machine_code: bytes          # The assembled binary
    symbol_table: dict[str, int] # Label → address mapping
    errors: list[AssemblyError]  # Any errors encountered
    source_map: dict[int, int]   # address → source line number (for debugging)

@dataclass
class AssemblyError:
    line: int
    message: str

# Individual encoding functions (testable independently)
def encode_data_processing(
    cond: int, opcode: int, s: bool, rn: int, rd: int, operand2: int, immediate: bool
) -> int: ...

def encode_branch(cond: int, offset: int) -> int: ...

def encode_memory(cond: int, load: bool, rn: int, rd: int, offset: int) -> int: ...
```

## Data Flow

```
Input:  Assembly source text (str)
Output: Machine code (bytes) + symbol table + source map + errors
```

## Test Strategy

- Encode individual instructions and verify against known ARM encodings
- Assemble a single instruction and verify output bytes
- Assemble a program with labels and verify label addresses are resolved correctly
- Assemble a program with forward references (label used before it's defined)
- Verify error reporting for invalid syntax, unknown instructions, invalid register names
- End-to-end: assemble `MOV R0, #1; MOV R1, #2; ADD R2, R0, R1; HLT`, load into ARM simulator, run, verify R2 = 3
- Roundtrip: disassemble the output and verify it matches the input

## Future Extensions

- **Disassembler**: Binary → assembly text (the reverse operation)
- **Macro support**: Define reusable assembly patterns
- **Linker**: Combine multiple assembly files, resolve cross-file references
- **Object file format**: Output structured binary (like ELF) instead of raw bytes
