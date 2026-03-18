# 08 — Bytecode Compiler

## Overview

The bytecode compiler walks the AST produced by the parser and emits a sequence of bytecode instructions for the virtual machine. It translates tree-structured code into a flat sequence of stack-machine operations.

This is Layer 8 of the computing stack. It depends on the parser package.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → [YOU ARE HERE] → VM
```

**Input from:** Parser (provides the AST).
**Output to:** Virtual machine (executes the bytecode).

Additionally, this compiler can optionally target the assembler (Layer 5) to emit ARM assembly instead of bytecode, enabling the full hardware path.

## Concepts

### What is bytecode?

Bytecode is a compact, flat representation of a program designed for a virtual machine. It is called "bytecode" because each instruction is typically one byte (the opcode) followed by zero or more argument bytes.

```python
# Source
x = 1 + 2

# AST
Assignment(Name("x"), BinaryOp(Number(1), "+", Number(2)))

# Bytecode
LOAD_CONST  1      # Push 1 onto the stack
LOAD_CONST  2      # Push 2 onto the stack
ADD                # Pop two values, add, push result
STORE_NAME  "x"    # Pop value, store in variable "x"
```

### Stack-based compilation

The compiler uses the AST node types to decide what instructions to emit:

```
visit(Number(n)):     emit LOAD_CONST n
visit(Name(id)):      emit LOAD_NAME id
visit(BinaryOp):      visit(left), visit(right), emit ADD/SUB/MUL/DIV
visit(Assignment):    visit(value), emit STORE_NAME target
visit(Print):         visit(value), emit PRINT
visit(If):            visit(condition), emit JUMP_IF_FALSE label, visit(body), label:
visit(While):         start: visit(condition), emit JUMP_IF_FALSE end, visit(body), emit JUMP start, end:
```

### Instruction set (MVP)

| Opcode | Arguments | Stack effect | Description |
|--------|-----------|-------------|-------------|
| LOAD_CONST | index | → value | Push a constant onto the stack |
| LOAD_NAME | index | → value | Push a variable's value onto the stack |
| STORE_NAME | index | value → | Pop value, store in variable |
| ADD | — | a, b → result | Pop two, push sum |
| SUB | — | a, b → result | Pop two, push difference |
| MUL | — | a, b → result | Pop two, push product |
| DIV | — | a, b → result | Pop two, push quotient |
| PRINT | — | value → | Pop value, print it |
| JUMP | offset | — | Unconditional jump |
| JUMP_IF_FALSE | offset | value → | Pop value, jump if falsy |
| COMPARE | op | a, b → result | Pop two, push comparison result |
| HALT | — | — | Stop execution |

### Compilation output

The compiler produces a `CodeObject` — a bundle containing:
- The bytecode instructions (list of opcodes + arguments)
- The constants table (numbers, strings used in the program)
- The names table (variable names used in the program)

This is exactly what CPython's `compile()` function produces.

## Public API

```python
class OpCode(Enum):
    LOAD_CONST = 0x01
    LOAD_NAME = 0x02
    STORE_NAME = 0x03
    ADD = 0x10
    SUB = 0x11
    MUL = 0x12
    DIV = 0x13
    PRINT = 0x20
    JUMP = 0x30
    JUMP_IF_FALSE = 0x31
    COMPARE = 0x32
    HALT = 0xFF

@dataclass
class Instruction:
    opcode: OpCode
    arg: int | None = None

@dataclass
class CodeObject:
    instructions: list[Instruction]
    constants: list[int | str]       # Constants pool
    names: list[str]                 # Variable names pool

class Compiler:
    def __init__(self) -> None: ...

    def compile(self, ast: Program) -> CodeObject: ...
        # Compile an AST into a CodeObject

    @property
    def errors(self) -> list[CompileError]: ...

@dataclass
class CompileError:
    message: str
    node: object   # The AST node where the error occurred

# Optional: ARM code generation (Path B)
class ARMCodeGenerator:
    def __init__(self) -> None: ...

    def generate(self, ast: Program) -> str: ...
        # Generate ARM assembly text from an AST
```

## Data Flow

```
Input:  AST (Program node from the parser)
Output: CodeObject (bytecode + constants + names)
```

## Test Strategy

- Compile a number literal: `42` → [LOAD_CONST 0], constants=[42]
- Compile a variable reference: `x` → [LOAD_NAME 0], names=["x"]
- Compile addition: `1 + 2` → [LOAD_CONST 0, LOAD_CONST 1, ADD]
- Compile assignment: `x = 1 + 2` → [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0]
- Compile print: `print(42)` → [LOAD_CONST 0, PRINT]
- Compile if statement: verify JUMP_IF_FALSE is emitted with correct offset
- Compile while loop: verify JUMP back and JUMP_IF_FALSE forward
- Verify constants deduplication (same constant used twice → one entry in constants pool)
- Verify names deduplication
- End-to-end: source → lexer → parser → compiler → code object → verify bytecode

## Future Extensions

- **Functions**: CALL, RETURN, frame management
- **Closures**: Captured variable handling
- **Optimization passes**: Constant folding (1 + 2 → 3 at compile time), dead code elimination
- **ARM backend**: Full code generation targeting the assembler
- **Bytecode serialization**: Save/load compiled code (like .pyc files)
