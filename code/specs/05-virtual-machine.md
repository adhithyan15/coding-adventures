# 09 — Virtual Machine

## Overview

The virtual machine (VM) is the runtime that executes bytecode produced by the compiler. It is a stack-based interpreter: a loop that reads instructions one at a time, manipulates a value stack, and maintains variable storage.

This is what CPython, Ruby's YARV, and Java's JVM do. The VM is the "engine" of the language.

This is Layer 9 of the computing stack — the top layer. It depends on the bytecode compiler.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → [YOU ARE HERE]
```

**Input from:** Bytecode compiler (provides CodeObject with instructions, constants, names).
**Output to:** The user — program output (printed values, return values).

## Concepts

### Stack-based execution

The VM maintains a stack of values. Instructions push values onto the stack and pop them off:

```
Instruction     Stack (top on right)     Notes
────────────    ────────────────────     ─────
                []                       Empty stack
LOAD_CONST 1    [1]                      Push 1
LOAD_CONST 2    [1, 2]                   Push 2
ADD             [3]                      Pop 2 and 1, push 3
STORE_NAME x    []                       Pop 3, store in x
```

### The eval loop

The heart of the VM is a simple loop:

```python
while True:
    instruction = fetch_next()
    match instruction.opcode:
        case LOAD_CONST: stack.push(constants[instruction.arg])
        case LOAD_NAME:  stack.push(variables[names[instruction.arg]])
        case STORE_NAME: variables[names[instruction.arg]] = stack.pop()
        case ADD:        b, a = stack.pop(), stack.pop(); stack.push(a + b)
        case PRINT:      print(stack.pop())
        case HALT:       break
```

This is the same pattern as the CPU's fetch-decode-execute cycle (Layer 3) and the ARM simulator's instruction dispatch (Layer 4). The concept is identical at every level — only the instruction set changes.

### Variable storage

Variables are stored in an "environment" — a dictionary mapping names to values:

```python
variables = {}
# STORE_NAME "x" → variables["x"] = stack.pop()
# LOAD_NAME  "x" → stack.push(variables["x"])
```

### Instruction pointer (IP)

The IP is the bytecode equivalent of the CPU's program counter. It tracks which instruction to execute next. Normally it increments by 1 after each instruction. JUMP instructions set it to a specific value.

### Runtime errors

The VM must handle:
- Stack underflow (popping from an empty stack)
- Undefined variable (LOAD_NAME for a variable that was never stored)
- Division by zero
- Type errors (adding a string and a number)

## Public API

```python
class VM:
    def __init__(self) -> None: ...

    def execute(self, code: CodeObject) -> VMResult: ...
        # Execute a compiled program

    @property
    def stack(self) -> list: ...
        # Current stack contents (for inspection/debugging)

    @property
    def variables(self) -> dict[str, object]: ...
        # Current variable bindings

    @property
    def output(self) -> list[str]: ...
        # Captured print output

@dataclass
class VMResult:
    output: list[str]              # Everything that was printed
    variables: dict[str, object]   # Final variable state
    error: VMError | None          # Runtime error, if any

@dataclass
class VMError:
    message: str
    instruction_index: int         # Which instruction caused the error
    instruction: Instruction       # The instruction itself

# Debugging support
@dataclass
class ExecutionTrace:
    steps: list[TraceStep]

@dataclass
class TraceStep:
    instruction_index: int
    instruction: Instruction
    stack_before: list
    stack_after: list
    variables: dict[str, object]

class DebugVM(VM):
    def execute_with_trace(self, code: CodeObject) -> tuple[VMResult, ExecutionTrace]: ...
        # Execute and record every step for debugging
```

## Data Flow

```
Input:  CodeObject (instructions + constants + names)
Output: VMResult (printed output + final variable state + any error)
```

## Test Strategy

- Execute LOAD_CONST: verify value is on the stack
- Execute LOAD_NAME / STORE_NAME: verify variable storage and retrieval
- Execute arithmetic: ADD, SUB, MUL, DIV with various values
- Execute PRINT: verify output capture
- Execute HALT: verify execution stops
- Execute JUMP: verify IP changes
- Execute JUMP_IF_FALSE: verify conditional branching
- Runtime error: undefined variable → clear error message
- Runtime error: stack underflow → clear error message
- Runtime error: division by zero → clear error message
- End-to-end: `x = 1 + 2` → source → lex → parse → compile → execute → variables["x"] == 3
- End-to-end: `print(1 + 2)` → source → lex → parse → compile → execute → output == ["3"]
- Trace mode: verify each step records correct stack state
- Compare output: run same program through Python's `exec()` and through our pipeline, verify same result

## Future Extensions

- **Functions and call frames**: CALL pushes a new frame, RETURN pops it
- **Garbage collection**: Track and free unused values
- **JIT compilation**: Detect hot loops, compile to native code
- **Debugger**: Step through bytecode, set breakpoints, inspect state
- **Cross-language VM**: Execute bytecode compiled from different source languages
