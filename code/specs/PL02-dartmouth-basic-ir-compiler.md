# PL02 — Dartmouth BASIC IR Compiler

## Overview

This spec defines the `dartmouth-basic-ir-compiler` Python package: the Dartmouth BASIC
frontend that lowers a parsed AST into a `compiler_ir.IrProgram`. The result is a
target-independent IR that any backend (GE-225, WASM, JVM) can translate to native code.

The package sits at layer 3 in the compiled pipeline:

```
BASIC source
    ↓  dartmouth-basic-lexer     (tokenize)
    ↓  dartmouth-basic-parser    (parse to AST)
    ↓  dartmouth-basic-ir-compiler   (this spec — lower to IR)
    ↓  ir-to-ge225-compiler         (spec IR01 — emit machine words)
    ↓  ge225-simulator               (execute)
```

Separation of concerns: this compiler knows nothing about GE-225 encoding. Its only job is
to translate BASIC semantics into target-independent IR instructions.

## V1 Scope

The first version supports a subset of 1964 Dartmouth BASIC sufficient to compile
and run meaningful integer programs:

| Statement  | Support |
|------------|---------|
| `REM`      | No-op (emits COMMENT) |
| `LET`      | Scalar variable assignment; all arithmetic operators (+, -, *, /, ^) |
| `PRINT`    | String literal only (e.g., `PRINT "HELLO"`) |
| `GOTO`     | Unconditional jump to line number |
| `IF … THEN`| All six relational operators (<, >, <=, >=, =, <>); jump to line number |
| `FOR … TO [STEP]` | Positive integer step; variable as loop counter |
| `NEXT`     | Matches innermost FOR |
| `END`      | Halt |
| `STOP`     | Halt |

**Excluded from V1** (planned for V2):
- GOSUB / RETURN
- DIM (arrays)
- INPUT / DATA / READ / RESTORE
- DEF FN (user-defined functions)
- PRINT with variables, commas, or semicolons (numeric output)
- Negative or variable STEP in FOR

## Prerequisites: Compiler-IR Extensions

Before this package can be built, `compiler_ir` must be extended with two new opcodes:

```python
MUL = 25   # dst = lhs * rhs (signed integer)
DIV = 26   # dst = lhs / rhs (integer quotient; rounds toward zero)
```

These are appended to the `IrOp` enum without changing existing opcode values, preserving
forward compatibility with all existing backends.

## Virtual Register Convention

BASIC operates on named variables, not hardware registers. This compiler maps every name
to a *virtual register index* and never touches physical registers — that is the backend's
job. Two categories of virtual registers exist:

### Variable Registers (fixed indices)

BASIC scalar variables are assigned fixed virtual register indices so that control flow
across loop iterations and GOTO targets always reads and writes the same register:

```
v0   — syscall argument (char code for PRINT; set before each SYSCALL 1)
v1   — BASIC variable A
v2   — BASIC variable B
...
v26  — BASIC variable Z
```

Letter+digit variables (A0–Z9) are assigned the next 260 indices (v27–v286) in
alphabetical order: `v27=A0, v28=A1, …, v36=A9, v37=B0, …`.

These indices are constants, not allocated on first use. The backend allocates a memory
word (spill slot) for every register that the program ever references.

### Expression Temporaries (dynamic)

Each intermediate value in an expression gets a fresh virtual register. A monotonically
increasing counter starts at `v287` (after all variable registers). Fresh registers are
never recycled, which keeps the IR simple and avoids the need for liveness analysis.

A moderately complex expression like `(A + B) * C - 1` would consume registers v287,
v288, v289, v290 (one per interior node: add, multiply, constant, subtract).

## Source Program Structure

The parser produces an `ASTNode(rule_name="program")` with children:

```
program
  line                     ← LINE_NUM token, optional statement, NEWLINE token
    LINE_NUM "10"
    statement
      let_stmt
        KEYWORD "LET"
        variable / NAME "X"
        EQ "="
        expr / term / power / unary / primary / NUMBER "5"
    NEWLINE "\n"
```

The compiler iterates `program.children`, each being a `line` node. For each `line`:
1. Read the `LINE_NUM` token → integer line number `N`.
2. Emit `LABEL _line_N` (pseudo-instruction; resolves GOTO targets).
3. If a `statement` child exists, dispatch to the appropriate handler.

The LABEL is emitted before the statement so that a `GOTO N` or `IF … THEN N` that targets
the current line jumps to the very beginning of that line's code.

## Prologue and Epilogue

**Prologue** (emitted before any line code):

```
LABEL _start
```

Plus, for the GE-225 target, the backend emits TON (typewriter on) as its own prologue.
The IR compiler itself has no prologue instructions beyond the `_start` label.

**Epilogue** (emitted after all lines):

```
HALT
```

`END` and `STOP` statements also emit `HALT`. If a program has no `END`, the epilogue
`HALT` catches control fall-through.

## Statement Compilation

### REM

```
COMMENT "remark text"
```

Produces no machine instructions on any backend.

---

### LET var = expr

```python
v_val = compile_expr(expr)          # expression result in fresh register
# variable register is fixed (e.g., v1 for A)
ADD_IMM v_var, v_val, 0             # copy: v_var = v_val + 0
```

The `ADD_IMM` with immediate 0 is the canonical register-to-register copy in this IR
(there is no dedicated MOVE opcode). For literals this is often followed immediately by an
optimizer fold, though V1 does not include an optimizer.

When the expression result is already in the variable's register (e.g., `LET A = A + 1`),
the copy is still emitted for uniformity; the backend will produce `LDA [spill_A]; ADO; STA [spill_A]`.

---

### PRINT "string"

Each character in the literal is converted at compile time to the GE-225 typewriter code
and printed via SYSCALL 1. PRINT always appends a carriage return.

```python
GE225_CODES = {
    '0': 0o00, '1': 0o01, '2': 0o02, '3': 0o03, '4': 0o04,
    '5': 0o05, '6': 0o06, '7': 0o07, '8': 0o10, '9': 0o11,
    '/': 0o13, 'A': 0o21, 'B': 0o22, 'C': 0o23, 'D': 0o24,
    'E': 0o25, 'F': 0o26, 'G': 0o27, 'H': 0o30, 'I': 0o31,
    '-': 0o33, '.': 0o40, 'J': 0o41, 'K': 0o42, 'L': 0o43,
    'M': 0o44, 'N': 0o45, 'O': 0o46, 'P': 0o47, 'Q': 0o50,
    'R': 0o51, '$': 0o53, ' ': 0o60, 'S': 0o62, 'T': 0o63,
    'U': 0o64, 'V': 0o65, 'W': 0o66, 'X': 0o67, 'Y': 0o70,
    'Z': 0o71,
}
CARRIAGE_RETURN_CODE = 0o37
```

For each character `ch`:

```
LOAD_IMM v0, GE225_CODES[ch.upper()]
SYSCALL 1
```

After all characters:

```
LOAD_IMM v0, CARRIAGE_RETURN_CODE   ; CR = 0o37
SYSCALL 1
```

Characters not in `GE225_CODES` raise a `CompileError` in V1. Lowercase letters are
uppercased before lookup (1964 BASIC was uppercase-only; lowercase in string literals is
tolerated as syntactic sugar).

---

### GOTO lineno

```
JUMP _line_N
```

If `_line_N` is never defined (the line doesn't exist in the program), the IR assembles
correctly but the backend's label-resolution pass raises a `CodeGenError`.

---

### IF expr1 relop expr2 THEN lineno

The six relational operators map to IR comparison opcodes:

| BASIC relop | IR | Notes |
|-------------|-----|-------|
| `<`  | `CMP_LT vDst, v_lhs, v_rhs` | |
| `>`  | `CMP_GT vDst, v_lhs, v_rhs` | |
| `<=` | `CMP_GT vDst, v_lhs, v_rhs` then `NOT` | Derived: LE = NOT GT |
| `>=` | `CMP_LT vDst, v_lhs, v_rhs` then `NOT` | Derived: GE = NOT LT |
| `=`  | `CMP_EQ vDst, v_lhs, v_rhs` | |
| `<>` | `CMP_NE vDst, v_lhs, v_rhs` | |

`NOT` is implemented as: `ADD_IMM vFlipped, vCmp, -1; AND_IMM vFlipped, vFlipped, 1`
(toggles the low bit, which is 0 or 1).

Full IF/THEN compilation:

```python
v_lhs = compile_expr(expr1)
v_rhs = compile_expr(expr2)

if relop == '<':
    v_cmp = new_reg(); CMP_LT v_cmp, v_lhs, v_rhs
elif relop == '>':
    v_cmp = new_reg(); CMP_GT v_cmp, v_lhs, v_rhs
elif relop == '=':
    v_cmp = new_reg(); CMP_EQ v_cmp, v_lhs, v_rhs
elif relop == '<>':
    v_cmp = new_reg(); CMP_NE v_cmp, v_lhs, v_rhs
elif relop == '<=':
    v_gt  = new_reg(); CMP_GT v_gt, v_lhs, v_rhs
    v_cmp = new_reg(); ADD_IMM v_cmp, v_gt, -1
                       AND_IMM v_cmp, v_cmp, 1
elif relop == '>=':
    v_lt  = new_reg(); CMP_LT v_lt, v_lhs, v_rhs
    v_cmp = new_reg(); ADD_IMM v_cmp, v_lt, -1
                       AND_IMM v_cmp, v_cmp, 1

BRANCH_NZ v_cmp, _line_N
```

`BRANCH_NZ cmp_reg, label` jumps to `label` when `cmp_reg != 0` (i.e., condition is true).

---

### FOR var = expr1 TO expr2 [STEP expr3]

FOR compiles to a pre-test loop: if the initial value already exceeds the limit, the body
is skipped entirely. This matches the historical 1964 BASIC semantics.

```python
# Evaluate expressions (may be complex expressions, not just literals)
v_start = compile_expr(expr1)
v_limit = compile_expr(expr2)
v_step  = compile_expr(expr3) if expr3 else new_reg() then LOAD_IMM v_step, 1

# Initialize loop variable
ADD_IMM v_var, v_start, 0          # var = start (copy)

# Save limit in its own register (persists across loop body)
# v_limit is already a fresh register, so it's stable across iterations

LABEL _for_N_check
# Pre-test: exit if var > limit (assumes positive step)
v_cmp = new_reg()
CMP_GT v_cmp, v_var, v_limit
BRANCH_NZ v_cmp, _for_N_end

# ... compile body statements ...

# Increment
ADD v_var, v_var, v_step           # var += step
JUMP _for_N_check

LABEL _for_N_end
```

`N` is a unique loop counter incremented each time a FOR is encountered. Nested loops
get different N values.

**NEXT var**: the compiler tracks a stack of open FOR loops. NEXT pops the innermost loop,
verifies the variable name matches (or raises `CompileError`), then emits:

```python
ADD v_var, v_var, v_step           # var += step
JUMP _for_N_check
LABEL _for_N_end
```

Wait — the increment and jump-back are emitted when NEXT is processed, not when FOR is.
The FOR statement emits the initialization and the check label; NEXT emits the increment,
backward jump, and end label. See the FOR stack below.

**FOR stack entry** pushed when FOR is encountered:

```python
@dataclass
class ForRecord:
    var_reg: int          # virtual register for the loop variable
    limit_reg: int        # virtual register holding the limit value
    step_reg: int         # virtual register holding the step value
    check_label: str      # label at the start of the pre-test
    end_label: str        # label after the loop (patch target)
    loop_num: int         # unique N for this loop
```

---

### END / STOP

```
HALT
```

---

## Expression Compilation

Expressions are compiled by recursively walking the AST. Each call to `compile_expr(node)`
returns the virtual register index holding the result. The grammar produces this structure:

```
expr   → term { (PLUS | MINUS) term }
term   → power { (STAR | SLASH) power }
power  → unary [CARET unary]
unary  → MINUS unary | primary
primary → variable | NUMBER | LPAREN expr RPAREN
```

### Primary

- **NUMBER**: `LOAD_IMM v_new, int(token.value)` — note: V1 truncates to integer. Float
  literals like `3.14` are truncated to `3` with a compile warning.
- **NAME (scalar variable)**: returns the variable's fixed register (no instruction emitted).
- **NAME(expr) (array element)**: not supported in V1 (raises `CompileError`).
- **LPAREN expr RPAREN**: recursively compiles `expr` and returns its register.

### Unary minus

```python
v_inner = compile_expr(unary.child)
v_result = new_reg()
LOAD_IMM v_result, 0
SUB v_result, v_result, v_inner    # result = 0 - inner = -inner
```

### Binary arithmetic

For each binary operator, compile both sides then emit one instruction:

| Operator | IR Opcode |
|----------|-----------|
| `+` | `ADD v_result, v_lhs, v_rhs` |
| `-` | `SUB v_result, v_lhs, v_rhs` |
| `*` | `MUL v_result, v_lhs, v_rhs` |
| `/` | `DIV v_result, v_lhs, v_rhs` |
| `^` | Not in V1 (raises `CompileError`) |

### Operator precedence

The grammar already encodes precedence via rule nesting — `term` wraps `power` which wraps
`unary` which wraps `primary`. No explicit precedence logic is needed in the compiler.

## Label Naming Conventions

| Purpose | Label |
|---------|-------|
| Entry point | `_start` |
| Line N | `_line_N` (e.g., `_line_100`) |
| FOR check (loop N) | `_for_N_check` |
| FOR end (loop N) | `_for_N_end` |

Labels beginning with `_` are compiler-generated. User labels (line numbers) use `_line_N`.

## Error Handling

The compiler raises `CompileError` (a subclass of `ValueError`) for:
- An unsupported statement type (e.g., GOSUB in V1)
- A NEXT without a matching FOR
- A FOR with a nested NEXT naming the wrong variable
- A character in a PRINT string that has no GE-225 typewriter code
- A power expression (^ operator)
- An array element reference

Undefined GOTO targets are **not** caught at compile time — they are left as dangling
`JUMP _line_N` instructions and will fail at backend link time.

## Compiler State

The internal `_Compiler` class tracks:

```python
@dataclass
class _Compiler:
    _program: IrProgram        # being built
    _next_reg: int             # next fresh register index (starts at 287)
    _loop_count: int           # unique loop counter for FOR label names
    _for_stack: list[ForRecord]  # stack of open FOR records
```

## Public API

```python
from dartmouth_basic_ir_compiler import compile_basic

def compile_basic(ast: ASTNode) -> CompileResult:
    """Lower a parsed BASIC AST to an IrProgram.

    Args:
        ast: Root ASTNode from dartmouth_basic_parser (rule_name == "program").

    Returns:
        CompileResult(program=IrProgram, var_regs=dict[str, int])

    Raises:
        CompileError: if the program uses any V1-excluded feature.
    """
```

`CompileResult.var_regs` maps BASIC variable names to virtual register indices —
useful for debugging and for tests that want to inspect final variable values.

## Example: Compiling a FOR Loop

BASIC source:

```
10 FOR I = 1 TO 3
20   PRINT "HI"
30 NEXT I
40 END
```

IR output (registers: I=v9, temps start at v287):

```
LABEL _start
LABEL _line_10
LOAD_IMM v287, 1          ; start
LOAD_IMM v288, 3          ; limit
LOAD_IMM v289, 1          ; step (default)
ADD_IMM  v9, v287, 0      ; I = start
LABEL _for_0_check
CMP_GT  v290, v9, v288    ; I > limit?
BRANCH_NZ v290, _for_0_end
LABEL _line_20
LOAD_IMM v0, 0o30         ; 'H' = 0o30
SYSCALL 1
LOAD_IMM v0, 0o31         ; 'I' = 0o31
SYSCALL 1
LOAD_IMM v0, 0o37         ; CR
SYSCALL 1
ADD     v9, v9, v289      ; I += step
JUMP    _for_0_check
LABEL _for_0_end
LABEL _line_40
HALT
```

## Package Structure

```
packages/python/dartmouth-basic-ir-compiler/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
└── src/
    └── dartmouth_basic_ir_compiler/
        ├── __init__.py           (exports compile_basic, CompileResult, CompileError)
        ├── compiler.py           (internal _Compiler class + compile_basic function)
        └── ge225_codes.py        (GE225_CODES dict + CARRIAGE_RETURN_CODE constant)
```

## Dependencies

```toml
[project]
dependencies = [
    "coding-adventures-compiler-ir",
    "coding-adventures-dartmouth-basic-parser",
]

[tool.uv.sources]
coding-adventures-compiler-ir = { path = "../compiler-ir", editable = true }
coding-adventures-dartmouth-basic-parser = { path = "../dartmouth-basic-parser", editable = true }
```

## Test Plan

- **REM**: verify COMMENT is emitted, no machine instructions in output.
- **LET with constant**: `LET A = 5` → `LOAD_IMM v1, 5; ADD_IMM v1, v1, 0` (or optimized copy).
- **LET with arithmetic**: `LET A = 2 + 3 * 4` → correct operator precedence.
- **GOTO**: `GOTO 100` → `JUMP _line_100`.
- **IF less-than**: verify `CMP_LT` + `BRANCH_NZ`.
- **IF less-or-equal**: verify the NOT pattern.
- **FOR default step**: v_step register initialized to 1.
- **FOR custom step**: v_step register initialized from expression.
- **FOR pre-test**: if start > limit, body labels appear but no body instructions execute.
- **PRINT string**: each char maps to correct GE-225 typewriter code; CR appended.
- **PRINT unsupported char**: raises `CompileError`.
- **END/STOP**: both emit `HALT`.
- **NEXT without FOR**: raises `CompileError`.
- **GOSUB**: raises `CompileError` (excluded in V1).
- **Round-trip line labels**: every LINE_NUM has a `LABEL _line_N` immediately before it.

Coverage target: 95%.
