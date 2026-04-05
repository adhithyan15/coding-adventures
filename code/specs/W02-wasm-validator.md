# W02 — WebAssembly Validator

## Overview

The WASM validator takes a parsed `WasmModule` (the output of `wasm-module-parser`,
spec W01) and checks that it is **semantically valid** — safe to execute without
undefined behavior. It is the gatekeeper between parsing and execution.

The parser (W01) checks *syntax*: is the binary well-formed? Are the section lengths
correct? Are the strings valid UTF-8? But it accepts many modules that are syntactically
parseable yet semantically broken — a function body that pops values that were never
pushed, a type index that refers to a non-existent entry, a branch to a label that does
not exist. The validator catches all of these.

A WASM module that passes validation carries a strong guarantee: **execution can never
encounter an operation that is undefined by the spec**. Every instruction has the right
operand types on the stack. Every index refers to something that exists. Every branch
target is reachable. This guarantee is what makes WASM safe to run in a sandbox.

```
  .wasm binary
       │
       ▼
  wasm-module-parser    ← checks syntax (binary format, UTF-8, lengths)
       │
       ▼
   WasmModule           ← structured in-memory representation
       │
       ▼
  wasm-validator        ← checks semantics (types, indices, stack balance)
       │
       ▼
  ValidatedModule       ← safe to hand to wasm-execution
```

## Why Validation Is a Separate Phase

It would be tempting to validate during execution — check each instruction as it runs.
This is wrong for two reasons:

**1. Security.** An execution engine that validates lazily can be caught mid-execution
when it detects a problem. At that point, the module has already run some code. For a
sandboxed runtime hosting untrusted modules, this is unacceptable.

**2. Performance.** Validating once upfront means the execution engine can skip
defensive checks on every instruction. A hot inner loop that runs a million times does
not need to re-verify stack types on each iteration. The validator's work pays off as
amortized zero-cost safety at execution time.

The WASM spec mandates this design: validation and execution are completely separate
passes. A conforming runtime must reject invalid modules before any code runs.

---

## Layer Position

```
     ┌─────────────────────────────────────────────────┐
     │  W02 — wasm-validator                           │
     │                                                 │
     │  Input:  WasmModule (from wasm-module-parser)   │
     │  Output: Ok(ValidatedModule) | Err(ValidationError) │
     │                                                 │
     │  Phase 1 — Structural validation                │
     │    • Cross-section reference checks             │
     │    • Limit and count constraints                │
     │    • Export uniqueness                          │
     │    • Constant expression validation             │
     │                                                 │
     │  Phase 2 — Type checking                        │
     │    • Per-function abstract stack machine        │
     │    • Control frame stack (block/loop/if)        │
     │    • Unreachable code handling                  │
     │    • All ~172 instruction type rules            │
     └────────────────────┬────────────────────────────┘
                          │ depends on
          ┌───────────────┼───────────────┐
          │               │               │
    wasm-types      wasm-opcodes   wasm-module-parser
```

---

## Public API

```python
from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class ValidatedModule:
    """A WasmModule that has passed full validation.

    This is a newtype wrapper — it carries the same data as WasmModule but
    its existence proves validation succeeded. The execution engine only
    accepts ValidatedModule, never raw WasmModule.

    The wrapper also caches derived data computed during validation that
    the execution engine needs — the total function count (imports +
    locals), the resolved type for each function, and the full local
    variable type list for each function body.
    """
    module: WasmModule

    # Derived during validation; cached for the execution engine:
    func_types: tuple[FuncType, ...]    # type of every function (imported + local)
    func_locals: tuple[tuple[ValueType, ...], ...]  # locals for each local function
                                                    # (params + declared locals)


class ValidationError(Exception):
    """Raised when a module fails validation.

    Carries both a human-readable message and a structured error kind so
    that the execution engine (and test suites) can match on specific
    failure modes without parsing strings.
    """
    def __init__(self, kind: "ValidationErrorKind", message: str):
        super().__init__(message)
        self.kind = kind
        self.message = message


class ValidationErrorKind(Enum):
    # ── Structural errors ──────────────────────────────────────────────
    INVALID_TYPE_INDEX         = "invalid_type_index"
    INVALID_FUNC_INDEX         = "invalid_func_index"
    INVALID_TABLE_INDEX        = "invalid_table_index"
    INVALID_MEMORY_INDEX       = "invalid_memory_index"
    INVALID_GLOBAL_INDEX       = "invalid_global_index"
    INVALID_LOCAL_INDEX        = "invalid_local_index"
    INVALID_LABEL_INDEX        = "invalid_label_index"
    INVALID_ELEMENT_INDEX      = "invalid_element_index"

    MULTIPLE_MEMORIES          = "multiple_memories"       # WASM 1.0: at most 1
    MULTIPLE_TABLES            = "multiple_tables"         # WASM 1.0: at most 1
    MEMORY_LIMIT_EXCEEDED      = "memory_limit_exceeded"   # max > 65536 pages
    MEMORY_LIMIT_ORDER         = "memory_limit_order"      # min > max
    TABLE_LIMIT_ORDER          = "table_limit_order"

    DUPLICATE_EXPORT_NAME      = "duplicate_export_name"
    EXPORT_INDEX_OUT_OF_RANGE  = "export_index_out_of_range"

    START_FUNCTION_BAD_TYPE    = "start_function_bad_type" # must be () → ()
    IMMUTABLE_GLOBAL_WRITE     = "immutable_global_write"  # global.set on const
    INIT_EXPR_INVALID          = "init_expr_invalid"       # bad constant expr

    # ── Type errors ───────────────────────────────────────────────────
    TYPE_MISMATCH              = "type_mismatch"     # wrong type on stack
    STACK_UNDERFLOW            = "stack_underflow"   # popped from empty stack
    STACK_HEIGHT_MISMATCH      = "stack_height_mismatch"  # wrong values at block end
    RETURN_TYPE_MISMATCH       = "return_type_mismatch"
    CALL_INDIRECT_TYPE_MISMATCH = "call_indirect_type_mismatch"


def validate(module: WasmModule) -> ValidatedModule:
    """Validate a parsed WasmModule.

    Runs both structural validation and type checking. Returns a
    ValidatedModule on success. Raises ValidationError on the first
    problem found.

    Args:
        module: A WasmModule produced by wasm-module-parser.

    Returns:
        ValidatedModule — proof that the module is safe to execute.

    Raises:
        ValidationError: With a structured kind and descriptive message.

    Example:
        >>> from wasm_module_parser import parse
        >>> from wasm_validator import validate, ValidationError
        >>>
        >>> module = parse(open("add.wasm", "rb").read())
        >>> try:
        ...     validated = validate(module)
        ...     print("Module is valid")
        ... except ValidationError as e:
        ...     print(f"Invalid: {e.kind} — {e.message}")
    """
```

---

## Phase 1: Structural Validation

Structural validation checks the module as a whole, verifying relationships between
sections before looking at any instruction bytecode. These checks are relatively cheap —
they are index lookups and count comparisons, not stack simulations.

### 1.1 Index Space Construction

WASM has five index spaces. Each index space is populated by both imports and local
definitions, in that order. Imports always come first.

```
Function index space:
  [0 .. num_imported_funcs)        ← imported functions
  [num_imported_funcs .. total)    ← locally defined functions

Table index space:
  [0 .. num_imported_tables)       ← imported tables
  [num_imported_tables .. total)   ← locally defined tables

Memory index space:
  [0 .. num_imported_memories)     ← imported memories
  [num_imported_memories .. total) ← locally defined memories

Global index space:
  [0 .. num_imported_globals)      ← imported globals
  [num_imported_globals .. total)  ← locally defined globals

Type index space:
  [0 .. num_types)                 ← only from the type section (no imports)
```

Constructing these index spaces is the first thing validation does. Every subsequent
check uses them.

```python
@dataclass
class IndexSpaces:
    """Resolved index spaces for a module."""
    func_types: list[FuncType]      # type of every function (imported + local)
    num_imported_funcs: int
    num_tables: int
    num_memories: int
    global_types: list[GlobalType]  # type of every global (imported + local)
    num_imported_globals: int
    num_types: int


def build_index_spaces(module: WasmModule) -> IndexSpaces:
    """Construct the index spaces from a parsed module.

    This is called first in validation. Every subsequent check indexes into
    these spaces rather than re-scanning sections.
    """
```

### 1.2 WASM 1.0 Cardinality Constraints

WASM 1.0 imposes strict limits on certain sections:

| Constraint | Rule | Error |
|---|---|---|
| At most 1 memory | `len(memories) + imported_memories ≤ 1` | `MULTIPLE_MEMORIES` |
| At most 1 table | `len(tables) + imported_tables ≤ 1` | `MULTIPLE_TABLES` |
| Memory max ≤ 65536 pages | `memory.limits.max ≤ 65536` if present | `MEMORY_LIMIT_EXCEEDED` |
| Memory min ≤ max | `memory.limits.min ≤ memory.limits.max` if both present | `MEMORY_LIMIT_ORDER` |
| Table min ≤ max | `table.limits.min ≤ table.limits.max` if both present | `TABLE_LIMIT_ORDER` |

65536 pages × 65536 bytes/page = 4 GiB — the maximum addressable memory in a 32-bit
address space. WASM 1.0 uses 32-bit addresses throughout.

### 1.3 Type Section

Each `FuncType` in the type section must use only valid value types. Since `ValueType`
is a closed enum (`I32`, `I64`, `F32`, `F64`), this check is whether the parser decoded
valid bytes. This is typically already guaranteed by the parser, but the validator
re-confirms it because the parser may be lenient.

### 1.4 Function Section

Each entry in the function section is a type index — it says "the Nth local function
has this signature." Validation checks that every type index is in bounds:

```
∀ type_idx in module.functions:
    type_idx < len(module.types)          → else INVALID_TYPE_INDEX
```

### 1.5 Import Section

Imported functions reference type indices:

```
∀ import in module.imports where import.kind == FUNCTION:
    import.type_info (as int) < len(module.types)   → else INVALID_TYPE_INDEX
```

Imported globals can be either mutable or immutable. Imported memories and tables are
checked against the same cardinality constraints as local ones.

### 1.6 Export Section

Two checks:

**Uniqueness:** No two exports may have the same name.
```
∀ (e1, e2) in module.exports where e1 ≠ e2:
    e1.name ≠ e2.name                     → else DUPLICATE_EXPORT_NAME
```

**Index validity:** Each export index must refer to an existing entity.
```
∀ export in module.exports:
    if export.kind == FUNCTION:  export.index < total_funcs
    if export.kind == TABLE:     export.index < total_tables
    if export.kind == MEMORY:    export.index < total_memories
    if export.kind == GLOBAL:    export.index < total_globals
```

### 1.7 Start Section

If present, the start function must have exactly the type `() → ()` — no parameters,
no return values. It is called automatically at instantiation.

```
if module.start is not None:
    func_type = resolve_func_type(module.start, index_spaces)
    func_type == FuncType(params=[], results=[])  → else START_FUNCTION_BAD_TYPE
```

### 1.8 Element Section

Element segments initialize tables at instantiation. Each segment specifies a table
index (must be 0 in WASM 1.0), a constant offset expression, and a list of function
indices.

```
∀ element in module.elements:
    element.table_index < total_tables          → else INVALID_TABLE_INDEX
    validate_const_expr(element.offset_expr, I32, index_spaces)
    ∀ func_idx in element.function_indices:
        func_idx < total_funcs                  → else INVALID_FUNC_INDEX
```

### 1.9 Data Section

Data segments initialize memory at instantiation. Each segment specifies a memory
index (must be 0 in WASM 1.0) and a constant offset expression.

```
∀ data in module.data:
    data.memory_index == 0 (WASM 1.0)           → else INVALID_MEMORY_INDEX
    validate_const_expr(data.offset_expr, I32, index_spaces)
```

### 1.10 Global Section

Each global has an init expression that must be a constant expression producing a
value of the declared type.

```
∀ global in module.globals:
    validate_const_expr(global.init_expr, global.global_type.value_type, index_spaces)
```

### 1.11 Constant Expressions

Constant expressions appear in global initializers, element offsets, and data offsets.
They are small bytecode sequences evaluated at instantiation time — before any function
runs. The WASM spec restricts them to a small, safe set of instructions:

```
Permitted constant expression instructions:
  i32.const <i32>         → pushes an i32 constant
  i64.const <i64>         → pushes an i64 constant
  f32.const <f32>         → pushes an f32 constant
  f64.const <f64>         → pushes an f64 constant
  global.get <globalidx>  → pushes the value of an imported global
                            (only imports; not locally defined globals)
  end                     → terminates the expression
```

A constant expression must end with exactly one value of the expected type on the
conceptual stack.

```python
def validate_const_expr(
    expr: bytes,
    expected_type: ValueType,
    index_spaces: IndexSpaces,
) -> None:
    """Validate a constant expression.

    Checks that:
    - Every instruction is in the permitted constant set
    - global.get only references imported globals (not local ones)
    - The expression produces exactly one value of expected_type
    - The expression ends with 'end' (0x0B)

    Raises:
        ValidationError(INIT_EXPR_INVALID): For any violation.
    """
```

---

## Phase 2: Type Checking

Type checking is the most algorithmically interesting part of validation. For each
function body, the validator runs an **abstract interpretation** — it simulates the
stack machine, but instead of tracking actual values (`42`, `3.14`, etc.), it tracks
their *types* (`I32`, `F64`, etc.).

### 2.1 The Abstract Stack Machine

Think of it as executing the function in a parallel universe where all values have
been replaced by their types. The stack contains types, not numbers. Instructions pop
types and push types.

```
Concrete execution (execution engine):
  i32.const 3    →  stack: [3]
  i32.const 5    →  stack: [3, 5]
  i32.add        →  stack: [8]

Abstract execution (type checker):
  i32.const 3    →  stack: [I32]
  i32.const 5    →  stack: [I32, I32]
  i32.add        →  stack: [I32]   (consumes 2 I32, produces 1 I32)
```

If the type checker encounters a type mismatch — say, `i32.add` but the top of the
stack is `F64` — it raises `TYPE_MISMATCH`. If it tries to pop from an empty stack, it
raises `STACK_UNDERFLOW`.

At the end of a function body, the stack must contain exactly the function's declared
return types. Nothing more, nothing less.

### 2.2 Local Variables

Each function has a set of local variables. Locals are numbered from 0. The first
locals are the function's parameters (their types come from the function's `FuncType`).
The remaining locals are the ones declared in the function body's local declarations.

```python
def build_func_locals(
    func_type: FuncType,
    body: FunctionBody,
) -> tuple[ValueType, ...]:
    """Build the complete local variable type list for a function.

    Index 0 .. len(params)-1      → parameter types (from FuncType)
    Index len(params) .. len-1    → declared locals (from FunctionBody)
    """
    return func_type.params + body.locals
```

The validator checks that every `local.get` and `local.set` index is within bounds:

```
∀ local.get  idx: idx < len(func_locals)   → else INVALID_LOCAL_INDEX
∀ local.set  idx: idx < len(func_locals)   → else INVALID_LOCAL_INDEX
∀ local.tee  idx: idx < len(func_locals)   → else INVALID_LOCAL_INDEX
```

And that the type matches:

```
local.get  idx: pushes func_locals[idx]
local.set  idx: pops exactly func_locals[idx]   → else TYPE_MISMATCH
local.tee  idx: pops func_locals[idx], pushes same
```

### 2.3 The Control Frame Stack

WASM uses *structured control flow*. There are no arbitrary jumps. Every branch target
is a lexically enclosing block: `block`, `loop`, or `if`. This is what makes type
checking tractable — you always know statically what label a branch resolves to.

The validator maintains a **control frame stack** alongside the value type stack.
Each frame records:

```python
@dataclass
class ControlFrame:
    """One entry in the control stack — one enclosing block/loop/if scope.

    kind:
        BLOCK — br jumps forward to the END of the block
        LOOP  — br jumps backward to the START of the loop
        IF    — br jumps forward to END; has an optional else branch

    start_types:
        The types expected on the value stack when entering this frame.
        For block/if: the block's parameter types (usually empty in WASM 1.0).
        For loop: the block's parameter types (loop targets re-consume them).

    end_types:
        The types that must be on the value stack when the block ends
        (or when br targets this frame from a block, not a loop).

    stack_height:
        The height of the value type stack when this frame was entered.
        Used to check that a br does not reach below this frame.

    unreachable:
        True after an unconditional branch, return, or unreachable instruction.
        In this state, the value type stack is treated as polymorphic — any
        pop is allowed (returning an Unknown type), any push is allowed.
        The frame becomes reachable again at the next 'end' or 'else'.
    """
    kind: Literal["block", "loop", "if"]
    start_types: tuple[ValueType, ...]
    end_types: tuple[ValueType, ...]
    stack_height: int
    unreachable: bool = False
```

The function itself is an implicit outermost frame — its `end_types` are the function's
declared return types.

### 2.4 How Labels Work

`br N` branches to the Nth enclosing label, where 0 is the immediately enclosing frame.
But what "branch" means depends on the frame kind:

```
frame kind    br targets          what must be on the stack
──────────────────────────────────────────────────────────
block         the END of block    frame's end_types
loop          the START of loop   frame's start_types (re-enter the loop)
if            the END of if       frame's end_types
```

This is a key asymmetry: `br` to a `loop` must have the loop's *input* types on the
stack (because control returns to the loop's start), while `br` to a `block` must have
the block's *output* types (because control exits the block).

```
                      ┌── block (result i32) ───────────┐
  outer scope         │                                  │
  ─────────────       │  i32.const 42                   │
                      │  br 0     ─────────────────────► │ end
                      │  (must have I32 on stack)        │
                      └──────────────────────────────────┘

                      ┌── loop (void) ──────────────────┐
  ◄─────────────────► │                                  │
  br 0 re-enters      │  ;; loop body                   │
  the loop here       │  br 0     ◄────────────────────  │
                      │  (must have void = [] on stack)  │
                      └──────────────────────────────────┘
```

### 2.5 The Unreachable State

After an instruction that unconditionally transfers control — `br`, `return`,
`unreachable` — any subsequent code up to the matching `end` or `else` is dead.
It can never be executed.

However, this dead code still appears in the bytecode and must be syntactically
processed. The WASM spec permits any stack configuration in unreachable code — type
checking is relaxed.

The way this is modeled: once a frame enters the unreachable state, any pop from
the type stack that would underflow returns a special `Unknown` type. An `Unknown` type
is compatible with any expected type. This allows dead code to "type check" regardless
of what it does.

```
Example:
  i32.const 1
  if (result i32)
    br 1               ;; branch out of if AND enclosing block — unreachable after
    f32.const 3.14     ;; dead code: validator marks frame as unreachable
    i64.add            ;; dead code: would be invalid if reachable, but OK in unreachable
  end                  ;; validator resets to reachable, stack should have I32
```

The unreachable state resets at `end` (exiting the block) and `else` (switching branch).

```python
def pop_type(
    stack: list[ValueType | Unknown],
    frame: ControlFrame,
    expected: ValueType,
) -> None:
    """Pop a value from the type stack and verify it matches expected.

    If the frame is in unreachable state and the stack is at or below the
    frame's entry height, the pop succeeds with Unknown (polymorphic).
    Otherwise, the top of stack must equal expected.
    """
    if frame.unreachable and len(stack) <= frame.stack_height:
        return  # Unknown — dead code, accept anything
    if not stack:
        raise ValidationError(STACK_UNDERFLOW, ...)
    actual = stack.pop()
    if actual != expected and actual is not Unknown:
        raise ValidationError(TYPE_MISMATCH, f"expected {expected}, got {actual}")
```

### 2.6 Instruction Type Rules

Every instruction has a type rule of the form: **[pop types] → [push types]**.

This section specifies the complete type rules. An instruction that references a type
index, function index, or local index must first validate that index is in bounds, then
derive its stack types from the referenced entity.

#### Control Instructions

| Instruction | Pops | Pushes | Notes |
|---|---|---|---|
| `unreachable` | (any) | (any) | Marks frame unreachable; traps at runtime |
| `nop` | — | — | No effect |
| `block t` | (start_types of t) | — | Push control frame (kind=BLOCK) |
| `loop t` | (start_types of t) | — | Push control frame (kind=LOOP) |
| `if t` | start_types + I32 | — | Push control frame (kind=IF); pop condition |
| `else` | end_types | start_types | Switch IF frame to ELSE branch |
| `end` | end_types | end_types | Pop control frame; push its end_types |
| `br L` | label_types(L) | (any) | Marks frame unreachable after branch |
| `br_if L` | label_types(L) + I32 | label_types(L) | Conditional; stack preserved if not taken |
| `br_table L* Ldef` | label_types(Ldef) + I32 | (any) | All targets must have same type; marks unreachable |
| `return` | func return types | (any) | Marks frame unreachable |
| `call F` | param_types(F) | result_types(F) | Index F must be valid |
| `call_indirect T` | param_types(T) + I32 | result_types(T) | T is a type index; pops table index |

For `br_table`, all label targets (including the default `Ldef`) must have compatible
label types. The table index is I32.

For `end`, the behavior depends on context:
- Exiting a `block` or `if`: value stack must contain exactly `end_types` above the
  frame's entry height.
- Ending a function body: value stack must contain exactly the function's return types.

#### Parametric Instructions

| Instruction | Pops | Pushes | Notes |
|---|---|---|---|
| `drop` | T | — | Any type |
| `select` | T T I32 | T | Both T must be the same type |

#### Variable Instructions

| Instruction | Pops | Pushes | Notes |
|---|---|---|---|
| `local.get L` | — | type_of(L) | L < len(func_locals) |
| `local.set L` | type_of(L) | — | L < len(func_locals) |
| `local.tee L` | type_of(L) | type_of(L) | L < len(func_locals) |
| `global.get G` | — | type_of(G) | G < total_globals |
| `global.set G` | type_of(G) | — | G must be mutable |

`global.set` on an immutable global raises `IMMUTABLE_GLOBAL_WRITE`.

#### Memory Instructions

Memory instructions require the module to have at least one memory (imported or local).
If no memory exists, these instructions are invalid.

| Instruction family | Pops | Pushes |
|---|---|---|
| `i32.load`, `i64.load`, `f32.load`, `f64.load` | I32 (address) | T |
| `i32.load8_s/u`, `i32.load16_s/u` | I32 | I32 |
| `i64.load8_s/u`, `i64.load16_s/u`, `i64.load32_s/u` | I32 | I64 |
| `i32.store`, `i64.store`, `f32.store`, `f64.store` | I32 T | — |
| `i32.store8`, `i32.store16` | I32 I32 | — |
| `i64.store8`, `i64.store16`, `i64.store32` | I32 I64 | — |
| `memory.size` | — | I32 |
| `memory.grow` | I32 | I32 |

The `memarg` immediate (alignment + offset) is read but only the alignment is
validated — it must not exceed the natural alignment of the access width:

```
i32.load / f32.load   → max alignment = 2 (2^2 = 4 bytes)
i64.load / f64.load   → max alignment = 3 (2^3 = 8 bytes)
i32.load8_s/u         → max alignment = 0 (2^0 = 1 byte)
i32.load16_s/u        → max alignment = 1 (2^1 = 2 bytes)
...etc.
```

#### Numeric Instructions

All numeric instructions only involve the four value types. The rule for each group:

| Group | Example | Pops | Pushes |
|---|---|---|---|
| i32 constant | `i32.const k` | — | I32 |
| i64 constant | `i64.const k` | — | I64 |
| f32 constant | `f32.const k` | — | F32 |
| f64 constant | `f64.const k` | — | F64 |
| i32 unary | `i32.clz`, `i32.ctz`, `i32.popcnt` | I32 | I32 |
| i32 binary | `i32.add`, `i32.sub`, ... `i32.rotr` | I32 I32 | I32 |
| i32 comparison unary | `i32.eqz` | I32 | I32 |
| i32 comparison binary | `i32.eq`, `i32.ne`, `i32.lt_s`, ... | I32 I32 | I32 |
| i64 unary | `i64.clz`, etc. | I64 | I64 |
| i64 binary | `i64.add`, etc. | I64 I64 | I64 |
| i64 comparison unary | `i64.eqz` | I64 | I32 (boolean) |
| i64 comparison binary | `i64.eq`, etc. | I64 I64 | I32 (boolean) |
| f32 unary | `f32.abs`, `f32.neg`, `f32.sqrt`, ... | F32 | F32 |
| f32 binary | `f32.add`, `f32.min`, `f32.copysign`, ... | F32 F32 | F32 |
| f32 comparison | `f32.eq`, `f32.lt`, ... | F32 F32 | I32 (boolean) |
| f64 unary | (mirror of f32) | F64 | F64 |
| f64 binary | (mirror of f32) | F64 F64 | F64 |
| f64 comparison | (mirror of f32) | F64 F64 | I32 (boolean) |

Note: comparison instructions always produce `I32` (0 = false, 1 = true) regardless
of whether they compare integers or floats.

#### Conversion Instructions

| Instruction | Pops | Pushes | Notes |
|---|---|---|---|
| `i32.wrap_i64` | I64 | I32 | Keep low 32 bits |
| `i32.trunc_f32_s`, `i32.trunc_f32_u` | F32 | I32 | Traps on NaN or out of range |
| `i32.trunc_f64_s`, `i32.trunc_f64_u` | F64 | I32 | Traps on NaN or out of range |
| `i64.extend_i32_s` | I32 | I64 | Sign-extend |
| `i64.extend_i32_u` | I32 | I64 | Zero-extend |
| `i64.trunc_f32_s`, `i64.trunc_f32_u` | F32 | I64 | Traps on NaN or out of range |
| `i64.trunc_f64_s`, `i64.trunc_f64_u` | F64 | I64 | Traps on NaN or out of range |
| `f32.convert_i32_s`, `f32.convert_i32_u` | I32 | F32 | |
| `f32.convert_i64_s`, `f32.convert_i64_u` | I64 | F32 | |
| `f32.demote_f64` | F64 | F32 | |
| `f64.convert_i32_s`, `f64.convert_i32_u` | I32 | F64 | |
| `f64.convert_i64_s`, `f64.convert_i64_u` | I64 | F64 | |
| `f64.promote_f32` | F32 | F64 | |
| `i32.reinterpret_f32` | F32 | I32 | Bit-cast (no conversion) |
| `i64.reinterpret_f64` | F64 | I64 | Bit-cast (no conversion) |
| `f32.reinterpret_i32` | I32 | F32 | Bit-cast (no conversion) |
| `f64.reinterpret_i64` | I64 | F64 | Bit-cast (no conversion) |

---

## The Validation Algorithm (Full Walkthrough)

This section traces through the complete validation algorithm for a single function.
Understanding this trace is the best preparation for implementing it.

Consider this function (in WAT text format):

```wat
;; Function signature: (i32, i32) → (i32)
;; Adds two numbers, but uses an if to return 0 if first arg is negative
(func (param i32) (param i32) (result i32)
  local.get 0       ;; push param 0
  i32.const 0
  i32.lt_s          ;; param0 < 0 ?
  if (result i32)   ;; enter if block, pops condition
    i32.const 0     ;; negative: return 0
  else
    local.get 0     ;; positive: return param0 + param1
    local.get 1
    i32.add
  end
)
```

**Initial state:**
```
func_locals = [I32, I32]   (params only, no declared locals)
func_return = [I32]
value_stack = []
control_stack = [ Frame(kind=FUNC, end_types=[I32], height=0) ]
```

**Instruction trace:**

```
local.get 0
  → push func_locals[0] = I32
  value_stack = [I32]

i32.const 0
  → push I32
  value_stack = [I32, I32]

i32.lt_s
  → pop I32, pop I32, push I32 (boolean result)
  value_stack = [I32]

if (result i32)
  → pop I32 (condition)
  → push control frame: kind=IF, start_types=[], end_types=[I32], height=0
  value_stack = []
  control_stack = [ Frame(FUNC,[I32],h=0), Frame(IF,[I32],h=0) ]

i32.const 0
  → push I32
  value_stack = [I32]

else
  → check: value_stack above height=0 == end_types=[I32] ✓
  → pop I32 from stack (the frame's end value)
  → switch IF frame to ELSE mode; reset stack to height=0
  value_stack = []

local.get 0
  → push I32
  value_stack = [I32]

local.get 1
  → push I32
  value_stack = [I32, I32]

i32.add
  → pop I32, pop I32, push I32
  value_stack = [I32]

end   (closes IF frame)
  → check: value_stack above height=0 == end_types=[I32] ✓
  → pop IF frame
  → push end_types = [I32]
  value_stack = [I32]
  control_stack = [ Frame(FUNC,[I32],h=0) ]

end   (closes FUNC frame — end of function)
  → check: value_stack == func_return=[I32] ✓
  → validation passes
```

---

## Public API (Complete)

```python
def validate(module: WasmModule) -> ValidatedModule:
    """Entry point. Runs Phase 1 then Phase 2."""

def validate_structure(module: WasmModule) -> IndexSpaces:
    """Phase 1 only. Returns index spaces on success."""

def validate_function(
    func_index: int,
    func_type: FuncType,
    body: FunctionBody,
    index_spaces: IndexSpaces,
    module: WasmModule,
) -> tuple[ValueType, ...]:
    """Phase 2 for a single function. Returns the full locals list.

    Useful for testing individual functions in isolation.
    """

def validate_const_expr(
    expr: bytes,
    expected_type: ValueType,
    index_spaces: IndexSpaces,
) -> None:
    """Validate a constant expression (global init, element/data offset)."""
```

---

## Test Strategy

### Valid Module Tests

These verify that correct modules pass without error.

| Test | Description |
|---|---|
| Empty module | No sections at all |
| Function with no locals | Simplest possible function body |
| Function returning void | `end` with empty stack |
| Function with all four value types | I32, I64, F32, F64 params and returns |
| Nested blocks | block inside block inside function |
| Loop with br | `br 0` targeting the loop start |
| `br_if` preserving stack | Conditional branch; stack intact on not-taken path |
| `br_table` | Multiple targets, all same type |
| `call` | Direct function call, types match |
| `call_indirect` | Indirect call via table |
| Memory load/store | With a declared memory |
| Global get/set | Mutable and immutable globals |
| All conversion instructions | One test per conversion |
| Start function | `() → ()` start function |
| Element and data segments | With valid constant expressions |
| Dead code after `br` | Unreachable state accepted |
| Dead code after `return` | Unreachable state accepted |
| Dead code after `unreachable` | Unreachable state accepted |

### Structural Error Tests

| Test | Expected Error |
|---|---|
| Two memory declarations | `MULTIPLE_MEMORIES` |
| Imported + local memory | `MULTIPLE_MEMORIES` |
| Memory max > 65536 | `MEMORY_LIMIT_EXCEEDED` |
| Memory min > max | `MEMORY_LIMIT_ORDER` |
| Function references type index out of range | `INVALID_TYPE_INDEX` |
| Export references function index out of range | `EXPORT_INDEX_OUT_OF_RANGE` |
| Duplicate export names | `DUPLICATE_EXPORT_NAME` |
| Start function with params | `START_FUNCTION_BAD_TYPE` |
| Start function with return value | `START_FUNCTION_BAD_TYPE` |
| global.set on imported immutable global | `IMMUTABLE_GLOBAL_WRITE` |
| Constant expression uses `i32.add` (not const) | `INIT_EXPR_INVALID` |
| Constant expression references local global | `INIT_EXPR_INVALID` |

### Type Error Tests

| Test | Expected Error |
|---|---|
| `i32.add` with F32 on stack | `TYPE_MISMATCH` |
| `i32.add` with only one value on stack | `STACK_UNDERFLOW` |
| Function returns I32 but stack has F64 | `RETURN_TYPE_MISMATCH` |
| `block (result i32)` ends with empty stack | `STACK_HEIGHT_MISMATCH` |
| `local.get 99` when only 2 locals exist | `INVALID_LOCAL_INDEX` |
| `local.set` with wrong type | `TYPE_MISMATCH` |
| `global.set` on immutable local global | `IMMUTABLE_GLOBAL_WRITE` |
| Memory instruction with no memory declared | `INVALID_MEMORY_INDEX` |
| `br 5` with only 3 frames on control stack | `INVALID_LABEL_INDEX` |
| `br_table` with targets of different types | `TYPE_MISMATCH` |
| `if (result i32)` then-branch ends with I64 | `TYPE_MISMATCH` |
| `else` branch ends with different type than `then` | `TYPE_MISMATCH` |

---

## Relationship to Execution

A `ValidatedModule` is the only input the execution engine (`wasm-execution`, spec W03)
accepts. The contract: **if validation passed, the execution engine may assume all type
checks hold and all indices are in bounds**. This means the execution engine can skip
bounds checks on local indices, global indices, type indices, and label depths — they
were all verified once, upfront, by the validator.

The one thing the validator cannot guarantee is **runtime traps** — these are conditions
that depend on values, not types:

| Runtime trap | Why validator cannot prevent it |
|---|---|
| Integer division by zero | Divisor is a runtime value, not known statically |
| Integer overflow in `i32.trunc_f32_s` | Float value is a runtime value |
| Memory access out of bounds | Address is a runtime value |
| `call_indirect` type mismatch | Function reference is resolved at runtime |
| `memory.grow` returning -1 | Host decides at runtime |

These traps are defined behavior — the spec says exactly what must happen (execution
stops, the host receives a trap signal). They are not undefined behavior. The execution
engine handles them explicitly.

---

## Dependencies

```
wasm-validator
    ├── wasm-module-parser   (provides WasmModule, FunctionBody, DataSegment, ...)
    ├── wasm-types           (provides ValueType, FuncType, BlockType, ...)
    └── wasm-opcodes         (provides OpcodeInfo for instruction metadata lookup)
```

No new external dependencies. The validator is pure logic over the data structures
defined in the packages below it.

---

## Package Names by Language

| Language | Parser package | Validator package |
|---|---|---|
| Rust | `wasm-module-parser` | `wasm-validator` |
| Python | `wasm_module_parser` | `wasm_validator` |
| Ruby | `wasm-module-parser` | `wasm-validator` |
| Go | `wasm-module-parser` | `wasm-validator` |
| TypeScript | `wasm-module-parser` | `wasm-validator` |
| Elixir | `wasm_module_parser` | `wasm_validator` |
| Perl | `WasmModuleParser` | `WasmValidator` |
