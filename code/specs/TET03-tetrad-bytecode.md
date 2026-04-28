# TET03 — Tetrad Bytecode Compiler Specification

> **LANG migration (2026-04-23):** Tetrad now also compiles to the
> generic `InterpreterIR` (LANG01) so the program can be executed on
> `vm-core` (LANG02) and JIT-compiled by `jit-core` (LANG03).  The
> translation lives in the `tetrad-runtime` package as
> `tetrad_runtime.compile_to_iir(source) -> IIRModule`.  The legacy
> `CodeObject` output described below remains the input to the
> translator and continues to be the contract `tetrad-vm` and
> `tetrad-jit` consume — nothing in this spec is invalidated; an extra
> downstream stage has been added.  See `tetrad-runtime/README.md` for
> the per-opcode translation table.

## Overview

The Tetrad bytecode compiler walks the AST produced by the parser (spec TET02) and
emits a register-based bytecode program. The output is a `CodeObject` — a self-contained
bundle of instructions, constants, and metadata needed to execute one function or the
top-level program.

The bytecode format follows the V8 Ignition model: an **accumulator** register (implicit,
always implied) plus 8 named general-purpose registers R0–R7. Most instructions read
from or write to the accumulator; the named register operand specifies the second operand.
This keeps instruction encoding small, which matters for fitting within 4 KB of 4004 ROM.

Every instruction that performs a binary operation or function call carries a **feedback
slot index**. The VM (spec TET04) writes type observations into these slots at runtime.
The JIT (spec TET05) reads them to decide what native code to emit.

---

## Instruction Set

### Encoding Overview

Instructions are variably-sized. Each instruction starts with a 1-byte opcode. The
number of operand bytes depends on the opcode:

```
Format 0: opcode (1 byte)             — no operands
Format 1: opcode reg (2 bytes)        — one register operand
Format 2: opcode imm8 (2 bytes)       — one 8-bit immediate
Format 3: opcode reg slot (3 bytes)   — register + feedback slot
Format 4: opcode idx (2 bytes)        — one pool index (constant / name / function)
Format 5: opcode offset (3 bytes)     — signed 16-bit jump offset
Format 6: opcode func argc (3 bytes)  — function index + argument count
```

Register operands are a 1-byte index (0–7 for R0–R7). Feedback slot operands are a
1-byte index into the function's feedback vector.

### 0x00–0x0F — Accumulator Loads

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x00 | `LDA_IMM imm8` | 2 | `acc = imm8` |
| 0x01 | `LDA_ZERO` | 0 | `acc = 0` (common fast path) |
| 0x02 | `LDA_REG r` | 1 | `acc = R[r]` |
| 0x03 | `LDA_VAR idx` | 4 | `acc = vars[idx]` |

`LDA_ZERO` is an optimization for the extremely common `let x = 0` pattern. On the 4004,
loading 0 has a dedicated path (CLB instruction) that saves one byte.

### 0x10–0x1F — Accumulator Stores

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x10 | `STA_REG r` | 1 | `R[r] = acc` |
| 0x11 | `STA_VAR idx` | 4 | `vars[idx] = acc` |

### 0x20–0x2F — Arithmetic (binary, acc ← acc OP R[r])

All arithmetic ops update the accumulator and record operand types in the feedback slot.

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x20 | `ADD r, slot` | 3 | `acc = (acc + R[r]) % 256` |
| 0x21 | `SUB r, slot` | 3 | `acc = (acc - R[r]) % 256` |
| 0x22 | `MUL r, slot` | 3 | `acc = (acc * R[r]) % 256` |
| 0x23 | `DIV r, slot` | 3 | `acc = acc / R[r]` (halt if R[r]==0) |
| 0x24 | `MOD r, slot` | 3 | `acc = acc % R[r]` (halt if R[r]==0) |
| 0x25 | `ADD_IMM imm8, slot` | 3 | `acc = (acc + imm8) % 256` (common fast path) |
| 0x26 | `SUB_IMM imm8, slot` | 3 | `acc = (acc - imm8) % 256` (common fast path) |

`ADD_IMM` / `SUB_IMM` avoid a register allocation for the common `n = n + 1` and
`n = n - 1` patterns. The compiler emits these when the right-hand operand is a literal.

### 0x30–0x3F — Bitwise Operations

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x30 | `AND r` | 1 | `acc = acc & R[r]` |
| 0x31 | `OR r` | 1 | `acc = acc \| R[r]` |
| 0x32 | `XOR r` | 1 | `acc = acc ^ R[r]` |
| 0x33 | `NOT` | 0 | `acc = (~acc) & 0xFF` |
| 0x34 | `SHL r` | 1 | `acc = (acc << R[r]) & 0xFF` |
| 0x35 | `SHR r` | 1 | `acc = acc >> R[r]` (logical, zero fill) |
| 0x36 | `AND_IMM imm8` | 2 | `acc = acc & imm8` (common: masking nibbles) |

Bitwise operations do not take a feedback slot because the operand types are always u8
in Tetrad v1. When a Lisp front-end introduces dynamic types, it will use a different
opcode range.

### 0x40–0x4F — Comparisons (result: 1 if true, 0 if false)

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x40 | `EQ r, slot` | 3 | `acc = 1 if acc == R[r] else 0` |
| 0x41 | `NEQ r, slot` | 3 | `acc = 1 if acc != R[r] else 0` |
| 0x42 | `LT r, slot` | 3 | `acc = 1 if acc < R[r] else 0` |
| 0x43 | `LTE r, slot` | 3 | `acc = 1 if acc <= R[r] else 0` |
| 0x44 | `GT r, slot` | 3 | `acc = 1 if acc > R[r] else 0` |
| 0x45 | `GTE r, slot` | 3 | `acc = 1 if acc >= R[r] else 0` |

Comparisons carry feedback slots because they are the primary polymorphic dispatch points
in a Lisp front-end (`equal?`, `<`, `>` dispatch on cons, number, symbol, etc.).

### 0x50–0x5F — Logical Operations

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x50 | `LOGICAL_NOT` | 0 | `acc = 0 if acc != 0 else 1` |
| 0x51 | `LOGICAL_AND r` | 1 | `acc = 0 if acc == 0 else (1 if R[r] != 0 else 0)` |
| 0x52 | `LOGICAL_OR r` | 1 | `acc = 1 if acc != 0 else (1 if R[r] != 0 else 0)` |

Note: The compiler implements short-circuit `&&` and `||` using jumps (JZ/JNZ), not
these instructions. `LOGICAL_AND` / `LOGICAL_OR` are available for cases where both
sides are already in registers without branching.

### 0x60–0x6F — Control Flow

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x60 | `JMP offset` | 5 | `ip += offset` (signed 16-bit relative) |
| 0x61 | `JZ offset` | 5 | `if acc == 0: ip += offset` |
| 0x62 | `JNZ offset` | 5 | `if acc != 0: ip += offset` |
| 0x63 | `JMP_LOOP offset` | 5 | backward jump (separate opcode for JIT loop detection) |

`JMP_LOOP` is functionally identical to `JMP` but uses a distinct opcode so the VM can
cheaply detect loop back-edges and increment the loop iteration counter for the hot-loop
detector (spec TET04).

Jump offsets are signed 16-bit values relative to the instruction following the jump.
An offset of 0 means "jump to the instruction immediately after this one" (no-op jump).

### 0x70–0x7F — Function Calls

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x70 | `CALL func_idx, argc, slot` | — | Push frame, jump to function |
| 0x71 | `RET` | 0 | Pop frame; return acc to caller |

`CALL` has a 4-byte encoding: opcode (1) + function pool index (1) + argument count
(1) + feedback slot (1). The function pool index refers to an entry in the `CodeObject`'s
`functions` array.

The feedback slot for `CALL` records the call site shape:
- `:monomorphic` — always calls the same function
- `:polymorphic` — calls 2–4 different functions
- `:megamorphic` — calls 5+ different functions (JIT gives up inlining)

### 0x80–0x8F — I/O

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0x80 | `IO_IN` | 0 | `acc = read_io_port()` |
| 0x81 | `IO_OUT` | 0 | `write_io_port(acc)` |

On a real 4004, `IO_IN` maps to the WRM/RDM instruction sequence that reads from a RAM
data port. `IO_OUT` maps to the WMP instruction that writes to output port latches.
The Python VM mocks these with stdin/stdout.

### 0xFF — VM Control

| Opcode | Mnemonic | Format | Effect |
|---|---|---|---|
| 0xFF | `HALT` | 0 | Stop execution |

---

## CodeObject Format

A `CodeObject` is the compiled form of one function or the top-level program.

```python
from tetrad_type_checker.types import FunctionTypeStatus

@dataclass
class CodeObject:
    name: str                       # function name or "<main>"
    params: list[str]               # parameter names (for debug/error messages)
    instructions: list[Instruction] # the bytecode
    constants: list[int]            # constant pool: u8 values
    var_names: list[str]            # variable name pool
    functions: list[CodeObject]     # nested function pool (callee CodeObjects)
    register_count: int             # how many registers this function needs (0–8)
    feedback_slot_count: int        # size of feedback vector to allocate at call time
                                    # 0 for FULLY_TYPED functions (no slots emitted)
    type_status: FunctionTypeStatus # FULLY_TYPED | PARTIALLY_TYPED | UNTYPED
    immediate_jit_eligible: bool    # True iff type_status == FULLY_TYPED
    source_map: list[tuple[int,int,int]]
    # source_map[i] = (instruction_offset, line, column)
    # maps bytecode offsets back to source positions for error messages
```

`immediate_jit_eligible` is a convenience flag derived from `type_status`. The VM and
JIT read this flag rather than re-checking the type status on every call.

### Instruction Format

```python
@dataclass
class Instruction:
    opcode: int               # 0x00–0xFF
    operands: list[int]       # 0, 1, 2, or 3 operand bytes depending on opcode
```

### Why Separate Pools?

**constants** holds u8 integer literals. `LDA_IMM` can embed values 0–255 directly in
the instruction stream (one byte operand), so the constant pool is only needed for
values that appear multiple times and benefit from pool deduplication.

**var_names** holds the string names of variables. On the 4004, names are never stored
at runtime (too much RAM); the compiler assigns each name a numeric index and the VM
uses the index. The names are kept in the CodeObject for debuggers and error messages.

**functions** holds nested CodeObjects. `CALL func_idx` uses the index into this array.
Top-level functions go into the main program's `functions` list.

---

## Two-Path Compilation (Typed vs Untyped)

The compiler receives a `TypeCheckResult` alongside the AST. For each binary operation,
it consults the type map to decide which instruction format to emit:

```
Op with both operands known u8 (from type map):
  → emit 2-byte instruction (opcode + register, NO slot)
  → feedback_slot_count unchanged

Op with at least one Unknown operand:
  → emit 3-byte instruction (opcode + register + slot index)
  → feedback_slot_count++
```

This is the **core payoff of the type checker**. A fully-typed function emits no slot
bytes anywhere in its body. On the 4004, this saves ROM and eliminates the feedback
vector RAM allocation entirely.

A FULLY_TYPED function also has `immediate_jit_eligible = True`, which the VM uses to
queue it for JIT compilation before it even runs once.

```python
def emit_binary_op(op: int, r: int, left_node: Expr, right_node: Expr,
                   type_map: dict, state: CompilerState):
    left_ty  = type_map.get(id(left_node), TypeInfo("Unknown")).ty
    right_ty = type_map.get(id(right_node), TypeInfo("Unknown")).ty
    if left_ty == "u8" and right_ty == "u8":
        # Statically typed — no feedback slot
        state.code.instructions.append(Instruction(op, [r]))
    else:
        # Dynamic — allocate and emit slot
        slot = state.next_slot
        state.next_slot += 1
        state.code.instructions.append(Instruction(op | 0x80, [r, slot]))
        # (The high bit convention differentiates slotted from non-slotted variants;
        #  the VM dispatch loop checks this bit to decide whether to record feedback)
```

Note: the instruction set (section above) lists both variants explicitly. The compiler
always emits the correct variant; no runtime switch is needed.

## Compiler Algorithm

The compiler maintains a `CompilerState` per function:

```python
@dataclass
class CompilerState:
    code: CodeObject              # being built
    locals: dict[str, int]        # name → var_names index for this scope
    next_register: int            # which register to allocate next (0–7)
    free_registers: list[int]     # registers available for reuse
    loop_starts: list[int]        # instruction offsets of enclosing while loop starts
    loop_end_patches: list[list[int]]  # lists of JMP offsets to patch on loop exit
```

### Expression Compilation

Expressions compile to a sequence of instructions that leaves the result in `acc`.
The key helper is `compile_expr(node) -> None`.

#### Integer Literal

```
node: IntLiteral(value=N)
→ emit LDA_IMM N          (or LDA_ZERO if N == 0)
```

#### Name Expression

Names can be in local variables or (for globals) in a global variable pool. The compiler
looks up the name in `locals` first, then in the global scope.

```
node: NameExpr(name='x')
→ emit LDA_VAR idx       where idx = var_names.index('x')
```

#### Binary Expression

```
node: BinaryExpr(op='+', left=L, right=R)

1. compile_expr(L)        → result in acc
2. emit STA_REG r         where r = allocate_register()
3. compile_expr(R)        → result in acc
   (right side is now in acc; left side in R[r])
4. swap acc and R[r]:
   emit STA_REG tmp_r     save right to tmp
   emit LDA_REG r         load left into acc
   emit ADD tmp_r, slot   acc = left + right
5. free_register(r)
6. free_register(tmp_r)
```

Wait — the accumulator already has R's value. The convention is:
**acc holds the left operand, R[r] holds the right operand** at the point of the binary
instruction. So the sequence is:

```
1. compile_expr(L)        → acc = left
2. emit STA_REG r_left    → R[r_left] = acc (save left)
3. compile_expr(R)        → acc = right
4. emit STA_REG r_right   → R[r_right] = acc (save right)
5. emit LDA_REG r_left    → acc = left
6. emit ADD r_right, slot → acc = left + right
7. free r_left, r_right
```

The compiler simplifies this when the right side is an integer literal:
```
1. compile_expr(L)        → acc = left
2. emit ADD_IMM N, slot   → acc = left + N
```

#### Short-Circuit Binary (&&, ||)

Short-circuit operators compile using conditional jumps, not the LOGICAL_AND/LOGICAL_OR
instructions:

```
a && b:
  compile_expr(a)       → acc = a
  JZ  (to false_label)  → if a is 0, skip b
  compile_expr(b)       → acc = b (result is b's truthiness)
  JMP (to end_label)
false_label:
  LDA_IMM 0             → acc = 0
end_label:
```

```
a || b:
  compile_expr(a)       → acc = a
  JNZ (to true_label)   → if a is non-zero, skip b
  compile_expr(b)       → acc = b
  JMP (to end_label)
true_label:
  LDA_IMM 1             → acc = 1
end_label:
```

#### Unary Expression

```
~x:
  compile_expr(x)       → acc = x
  emit NOT              → acc = ~acc & 0xFF

!x:
  compile_expr(x)       → acc = x
  emit LOGICAL_NOT      → acc = (acc==0 ? 1 : 0)

-x:
  compile_expr(x)       → acc = x
  emit STA_REG r
  emit LDA_ZERO
  emit SUB r, slot      → acc = 0 - x (wrapping negation)
```

#### Call Expression

```
f(arg1, arg2):
  compile_expr(arg1)    → acc = arg1
  STA_REG r0
  compile_expr(arg2)    → acc = arg2
  STA_REG r1
  CALL func_idx, 2, slot
  (result is in acc after return)
```

Arguments are evaluated left-to-right and placed in registers R0..R(argc-1). The
callee reads its parameters from those same registers.

#### in() and out(expr)

```
in():
  emit IO_IN

out(expr):
  compile_expr(expr)
  emit IO_OUT
```

### Statement Compilation

#### Let Statement

```
let x = expr;
→ compile_expr(expr)
→ idx = add_var_name('x')
→ emit STA_VAR idx
```

#### Assign Statement

```
x = expr;
→ compile_expr(expr)
→ idx = var_names.index('x')  (must already exist)
→ emit STA_VAR idx
```

#### If Statement

```
if cond { then } else { else }

→ compile_expr(cond)
→ emit JZ  (patch_1: offset unknown)
→ compile_block(then)
→ emit JMP (patch_2: offset unknown)
→ patch_1 to here
→ compile_block(else_block)  (or nothing if no else)
→ patch_2 to here
```

#### While Statement

```
while cond { body }

loop_start:
→ compile_expr(cond)
→ emit JZ  (patch_exit: offset unknown)
→ compile_block(body)
→ emit JMP_LOOP (back to loop_start)
→ patch_exit to here
```

`JMP_LOOP` is used for the backward jump so the VM can count loop iterations.

#### Return Statement

```
return expr;
→ compile_expr(expr)
→ emit RET

return;
→ emit LDA_ZERO
→ emit RET
```

### Jump Patching

Forward jumps use a two-pass approach:

```
1. emit_jump(opcode) → returns instruction index
2. ... compile intervening code ...
3. patch_jump(index) → fills in the offset from step 1 to current position
```

The offset is relative to the instruction following the jump (standard convention).

---

## Register Allocation

Tetrad uses a simple linear allocator. Each function gets registers R0–R7. The
compiler tracks which registers are in use and allocates the next free one. Registers
are freed when their holding value is no longer needed.

```python
def allocate_register() -> int:
    if free_registers:
        return free_registers.pop()
    r = next_register
    next_register += 1
    if next_register > 7:
        raise CompilerError("register spill: expression too complex (max 8 registers)")
    return r

def free_register(r: int):
    free_registers.append(r)
```

Register spill (needing more than 8 registers) is a `CompilerError`. In practice,
Tetrad's expression grammar cannot produce more than 8 live values simultaneously,
but deeply nested calls could hit this limit. If so, the user must split the expression.

---

## Feedback Slot Assignment

The compiler assigns feedback slot indices at emit time. A counter `next_slot` starts
at 0 for each function and increments each time an instruction needs a slot:

```python
def emit_binary_op(opcode: int, r: int) -> int:
    slot = next_slot
    next_slot += 1
    emit(opcode, [r, slot])
    return slot
```

The total `next_slot` value at the end of compilation is stored as
`feedback_slot_count` in the `CodeObject`. The VM allocates a vector of that size at
function call time.

---

## Compile-Time Checks

The compiler rejects:

| Condition | Error |
|---|---|
| Integer literal > 255 | `integer literal N out of u8 range (0–255)` |
| Hex literal > 0xFF | `hex literal 0xN out of u8 range` |
| Reference to undeclared variable | `undefined variable 'name'` |
| Call to undeclared function | `undefined function 'name'` |
| Call with wrong number of arguments | `'name' expects M args, got N` |
| Register spill | `expression too complex: exceeds 8 virtual registers` |
| `return` outside function | `'return' outside function` |

---

## CodeObject Binary Serialization (optional, for 4004 ROM)

When targeting the physical 4004, the CodeObject is serialized to bytes for embedding
in the ROM image alongside the interpreter. The serialization format is:

```
[1 byte]  function_count (number of functions including main)
For each function:
  [1 byte]  param_count
  [1 byte]  register_count
  [1 byte]  feedback_slot_count
  [2 bytes] instruction_count (little-endian)
  [N bytes] instructions (raw bytes, variable length)
  [1 byte]  var_count
  [1 byte]  const_count
  [M bytes] constants (each 1 byte, u8)
```

Variable names are not serialized for the ROM image (no RAM to store strings). The
var_names list is used only in the Python VM for debugging.

---

## Python Package

The compiler lives in `code/packages/python/tetrad-compiler/`.

Depends on `coding-adventures-tetrad-lexer` and `coding-adventures-tetrad-parser`.

### Public API

```python
from tetrad_compiler import compile_program, compile_checked, CompilerError
from tetrad_compiler.bytecode import CodeObject, Instruction

# Lex + parse + type-check + compile in one call.
# Raises LexError, ParseError, TypeError, or CompilerError on failure.
def compile_program(source: str) -> CodeObject: ...

# Compile from a pre-built TypeCheckResult (preferred — avoids re-running the checker).
# Raises CompilerError if TypeCheckResult.errors is non-empty.
def compile_checked(result: TypeCheckResult) -> CodeObject: ...

class CompilerError(Exception):
    def __init__(self, message: str, line: int, column: int): ...
```

---

## Test Strategy

### Instruction emission tests

- `let x = 0;` → `[LDA_ZERO, STA_VAR 0, HALT]`
- `let x = 42;` → `[LDA_IMM 42, STA_VAR 0, HALT]`
- `x = x + 1;` → involves `LDA_VAR`, `ADD_IMM 1`, `STA_VAR`
- `a + b * c` → multiply compiled before add (precedence from parser)

### Feedback slot tests

- `a + b` → emitted `ADD` carries slot 0
- Two binary ops → slots 0 and 1 respectively
- `feedback_slot_count` equals number of slotted instructions

### Control flow tests

- `if cond { body }` → verify `JZ` target skips body
- `if cond { a } else { b }` → verify `JZ` + `JMP` structure
- `while cond { body }` → verify backward `JMP_LOOP` and forward `JZ`

### Call tests

- `fn add(a, b) { return a + b; }` compiles to a CodeObject with 2 params
- `add(1, 2)` emits `LDA_IMM 1, STA_REG 0, LDA_IMM 2, STA_REG 1, CALL 0 2 slot`

### Error tests

- `let x = 300;` → `CompilerError` (overflow)
- `let y = x + 1;` where `x` undeclared → `CompilerError`
- `f(1, 2, 3)` where `f` declared with 2 params → `CompilerError`

### End-to-end tests

Compile + execute all five TET00 example programs and verify output matches expected.

### Coverage target

95%+ line coverage.

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
