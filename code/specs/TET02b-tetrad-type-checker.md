# TET02b — Tetrad Type Checker Specification

## Overview

The Tetrad type checker sits between the parser (spec TET02) and the bytecode compiler
(spec TET03). It walks the AST produced by the parser, resolves type annotations,
infers types where possible, catches type inconsistencies, and annotates every
expression node with its inferred type.

The output of the type checker — a `TypeCheckResult` containing the enriched AST and a
`TypeEnvironment` — is passed directly to the compiler. The compiler reads the type
annotations to decide whether to emit feedback slots (for untyped ops) or skip them
(for statically-known ops). This is the mechanism by which optional types accelerate
both the VM and the JIT.

Tetrad uses **gradual typing**: type annotations are optional. A program with no
annotations is valid and behaves exactly as in TET03/TET04 v1. A fully-annotated
program allows the compiler to produce a more compact bytecode with no feedback overhead.

---

## Motivation: Why Types Make the Pipeline Faster

```
Untyped:  fn add(a, b)      { return a + b; }
Typed:    fn add(a: u8, b: u8) -> u8 { return a + b; }
```

For the **untyped** function, the pipeline must:
1. Emit `ADD r0, slot=N` — 3 bytes (includes slot operand)
2. Allocate a feedback vector at call time — RAM cost
3. Record `u8 × u8` into slot N on every execution
4. Wait for 100 calls before the JIT compiles it

For the **typed** function, the pipeline can:
1. Emit `ADD r0` — 2 bytes (no slot operand needed)
2. Skip feedback vector allocation entirely — saves RAM
3. Mark the function `immediate_jit_eligible = True`
4. JIT compile it on the **first call**

On the Intel 4004, saving 1 byte per binary op is meaningful. A function with 5
arithmetic operations saves 5 bytes of ROM. Eliminating feedback vector allocation
saves RAM proportional to the feedback slot count.

For a future Lisp front-end, this matters even more: `(declare (fixnum x y))` annotates
a Lisp function with concrete types. The Tetrad type checker converts those into
`FULLY_TYPED` CodeObjects, and the JIT generates specialized integer code immediately
without any profiling warmup.

---

## The Three-Tier Type Status

Every function in a Tetrad program is classified into one of three tiers:

```
FULLY_TYPED      All parameters annotated, return type annotated,
                 all operations infer to a known type with no
                 contradictions. Compiler emits no feedback slots.
                 JIT compiles on first call.

PARTIALLY_TYPED  Some annotations present. The type checker annotates
                 what it can; the rest remains unknown. Compiler emits
                 feedback slots only for ops with unknown operand types.
                 JIT uses a lower threshold (10 calls).

UNTYPED          No annotations whatsoever. Identical to Tetrad v1
                 behavior. All ops get feedback slots. JIT uses the
                 standard threshold (100 calls).
```

A function is `PARTIALLY_TYPED` if it has at least one annotation but is not
`FULLY_TYPED`. A function with no annotations at all is `UNTYPED`.

---

## Type Language

In Tetrad v1, there is exactly one concrete type:

| Type token | Meaning |
|---|---|
| `u8` | Unsigned 8-bit integer, 0–255, wraps on overflow |

The type checker also uses two internal pseudo-types that do not appear in source:

| Internal type | Meaning |
|---|---|
| `Unknown` | No annotation and no inference was possible |
| `Void` | Used for `out(expr)` which has no meaningful value |

When a Lisp front-end is added, the concrete type list will expand:
`u8`, `pair`, `symbol`, `closure`, `bool`, `nil`. The type checker infrastructure
is designed to accommodate this without structural change.

---

## AST Enrichment

The type checker does not produce a new AST type hierarchy. Instead, it produces a
**TypeMap** — a mapping from AST node identity (`id(node)`) to `TypeInfo` — alongside
the original AST.

```python
@dataclass
class TypeInfo:
    ty: str                  # "u8", "Unknown", "Void"
    source: str              # "annotation" | "inferred" | "unknown"
    line: int                # source position for error messages
    column: int
```

The compiler passes `(ast, type_map)` to every compilation function, reading
`type_map[id(expr)]` to determine whether a slot is needed.

---

## Data Structures

### TypeEnvironment

```python
@dataclass
class FunctionType:
    param_types: list[str | None]   # None = unannotated parameter
    return_type: str | None         # None = unannotated return

@dataclass
class TypeEnvironment:
    # Function signatures visible at the call site
    functions: dict[str, FunctionType]

    # Variable types in scope (populated per-function during checking)
    variables: dict[str, TypeInfo]

    # Classification of each function
    function_status: dict[str, FunctionTypeStatus]

    def lookup_var(self, name: str) -> TypeInfo | None: ...
    def bind_var(self, name: str, info: TypeInfo) -> None: ...
    def child_scope(self) -> TypeEnvironment: ...   # for block scopes
```

### FunctionTypeStatus

```python
from enum import Enum

class FunctionTypeStatus(Enum):
    FULLY_TYPED     = "fully_typed"
    PARTIALLY_TYPED = "partially_typed"
    UNTYPED         = "untyped"
```

### TypeCheckResult

```python
@dataclass
class TypeCheckResult:
    program: Program                    # original AST (unchanged)
    type_map: dict[int, TypeInfo]       # id(expr node) → TypeInfo
    env: TypeEnvironment                # final type environment
    errors: list[TypeError]             # hard errors (compilation should abort)
    warnings: list[TypeWarning]         # soft warnings (compilation proceeds)
```

### TypeError and TypeWarning

```python
@dataclass
class TypeError:
    message: str
    line: int
    column: int

@dataclass
class TypeWarning:
    message: str
    line: int
    column: int
    hint: str = ""
```

---

## Type Inference Rules

The type checker walks the AST bottom-up, inferring the type of each expression from
the types of its sub-expressions and any available annotations.

### Literals

| Expression | Inferred type |
|---|---|
| `IntLiteral(N)` | `u8` — always (range check is compiler's job) |

### Variables

| Expression | Inferred type |
|---|---|
| `NameExpr(name)` | Look up `name` in current environment. If found and has type → use it. Else `Unknown`. |

### Arithmetic and Bitwise Binary Expressions

Tetrad's type algebra for u8 is closed: `u8 OP u8 → u8`. Any unknown operand
propagates Unknown upward.

| Left type | Right type | Result type |
|---|---|---|
| `u8` | `u8` | `u8` |
| `u8` | `Unknown` | `Unknown` |
| `Unknown` | `u8` | `Unknown` |
| `Unknown` | `Unknown` | `Unknown` |

### Comparison Expressions (`==`, `!=`, `<`, etc.)

Comparisons produce a boolean encoded as `u8` (0 or 1). The type checker treats the
result as `u8` regardless of operand types (since the result is always 0 or 1).

| Left type | Right type | Result type |
|---|---|---|
| any | any | `u8` |

### Logical Expressions (`&&`, `||`, `!`)

Result is always `u8` (0 or 1).

### Call Expressions

```
CallExpr(name, args)
  → result type = env.functions[name].return_type ?? Unknown
```

If the callee has no return type annotation, the result is `Unknown`.

### in()

`InExpr` → type is `Unknown` (value comes from hardware I/O at runtime)

### out(expr)

`OutExpr` → type is `Void` (side effect, no value)

---

## Function Status Classification Algorithm

After checking all expressions in a function, the type checker classifies it:

```python
def classify_function(fn: FnDecl, env: TypeEnvironment) -> FunctionTypeStatus:
    has_any_annotation = (
        any(t is not None for t in fn.param_types) or
        fn.return_type is not None
    )

    if not has_any_annotation:
        return FunctionTypeStatus.UNTYPED

    all_params_typed = all(t is not None for t in fn.param_types)
    return_typed = fn.return_type is not None

    if not (all_params_typed and return_typed):
        return FunctionTypeStatus.PARTIALLY_TYPED

    # All params and return are annotated.
    # Check that all ops inside the body inferred to known types.
    all_ops_typed = all(
        type_map[id(expr)].ty != "Unknown"
        for expr in all_expressions_in(fn.body)
        if is_binary_op(expr) or is_call(expr)
    )

    return FunctionTypeStatus.FULLY_TYPED if all_ops_typed else FunctionTypeStatus.PARTIALLY_TYPED
```

A function can be `FULLY_TYPED` only if every internal operation has a known type.
If any operation yields `Unknown` (e.g., because it calls an untyped function), the
function is at best `PARTIALLY_TYPED`.

---

## Type Checking Algorithm

```python
def check_program(program: Program) -> TypeCheckResult:
    type_map: dict[int, TypeInfo] = {}
    errors: list[TypeError] = []
    warnings: list[TypeWarning] = []

    # Phase 1: Collect all function signatures (forward declarations)
    env = TypeEnvironment(functions={}, variables={}, function_status={})
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            env.functions[decl.name] = FunctionType(
                param_types=decl.param_types,
                return_type=decl.return_type,
            )

    # Phase 2: Check globals
    for decl in program.decls:
        if isinstance(decl, GlobalDecl):
            inferred = check_expr(decl.value, env, type_map, errors)
            annotated = decl.declared_type
            if annotated and inferred.ty != "Unknown" and annotated != inferred.ty:
                errors.append(TypeError(
                    f"global '{decl.name}': declared {annotated}, got {inferred.ty}",
                    decl.line, decl.column
                ))
            actual_type = annotated or inferred.ty
            env.bind_var(decl.name, TypeInfo(ty=actual_type, source="annotation" if annotated else inferred.source, ...))

    # Phase 3: Check each function body
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            check_fn(decl, env, type_map, errors, warnings)

    # Phase 4: Classify each function
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            status = classify_function(decl, env, type_map)
            env.function_status[decl.name] = status
            if status == FunctionTypeStatus.UNTYPED:
                warnings.append(TypeWarning(
                    f"'{decl.name}' has no type annotations — JIT warmup required",
                    decl.line, decl.column,
                    hint="add param types and -> return type to enable immediate JIT compilation"
                ))

    return TypeCheckResult(program=program, type_map=type_map, env=env,
                           errors=errors, warnings=warnings)
```

### check_fn

```python
def check_fn(fn: FnDecl, env: TypeEnvironment, type_map, errors, warnings):
    local_env = env.child_scope()

    # Bind parameters with their annotated types (or Unknown)
    for name, ann_type in zip(fn.params, fn.param_types):
        ty = ann_type if ann_type else "Unknown"
        local_env.bind_var(name, TypeInfo(ty=ty, source="annotation" if ann_type else "unknown", ...))

    # Check the body
    check_block(fn.body, local_env, fn.return_type, type_map, errors, warnings)
```

### check_expr (bottom-up walk)

```python
def check_expr(expr, env, type_map, errors) -> TypeInfo:
    if isinstance(expr, IntLiteral):
        info = TypeInfo(ty="u8", source="inferred", ...)
    elif isinstance(expr, NameExpr):
        found = env.lookup_var(expr.name)
        info = found if found else TypeInfo(ty="Unknown", source="unknown", ...)
    elif isinstance(expr, BinaryExpr):
        left_info  = check_expr(expr.left, env, type_map, errors)
        right_info = check_expr(expr.right, env, type_map, errors)
        result_ty = "u8" if left_info.ty == "u8" and right_info.ty == "u8" else "Unknown"
        info = TypeInfo(ty=result_ty, source="inferred", ...)
    elif isinstance(expr, CallExpr):
        fn_type = env.functions.get(expr.name)
        result_ty = fn_type.return_type if fn_type and fn_type.return_type else "Unknown"
        info = TypeInfo(ty=result_ty, source="inferred" if result_ty != "Unknown" else "unknown", ...)
    elif isinstance(expr, InExpr):
        info = TypeInfo(ty="Unknown", source="unknown", ...)  # runtime I/O
    elif isinstance(expr, OutExpr):
        check_expr(expr.value, env, type_map, errors)
        info = TypeInfo(ty="Void", source="inferred", ...)
    elif isinstance(expr, UnaryExpr):
        operand_info = check_expr(expr.operand, env, type_map, errors)
        info = TypeInfo(ty=operand_info.ty, source="inferred", ...)
    elif isinstance(expr, GroupExpr):
        info = check_expr(expr.expr, env, type_map, errors)
    else:
        info = TypeInfo(ty="Unknown", source="unknown", ...)

    type_map[id(expr)] = info
    return info
```

---

## Errors and Warnings

### Hard Errors (abort compilation)

| Condition | Error message |
|---|---|
| `let x: u8 = in()` | `'x' declared u8 but assigned Unknown (I/O value has no static type)` |
| `fn f(a: u8) -> u8` body returns expression of Unknown type | `'f' declared -> u8 but return expression has unknown type` |
| Type annotation uses an unknown type name | `unknown type 'foo' at line N col C` |

### Soft Warnings (compilation proceeds)

| Condition | Warning message |
|---|---|
| Function has no annotations | `'f' is untyped — JIT requires warmup; add types for immediate compilation` |
| Calling an untyped function from a typed context | `call to untyped 'g' in typed context 'f' — 'f' downgraded to PARTIALLY_TYPED` |
| Parameter annotated but return type missing | `'f' has typed params but no return type — classified PARTIALLY_TYPED` |

---

## Interaction with the Compiler (TET03)

The compiler receives `(program: Program, result: TypeCheckResult)` and uses it as
follows:

```python
# When compiling a binary op, consult the type map:
def emit_binary_op(op, left_node, right_node, type_map, code):
    left_ty  = type_map[id(left_node)].ty
    right_ty = type_map[id(right_node)].ty

    if left_ty == "u8" and right_ty == "u8":
        # Both operands statically known — no feedback slot needed
        emit(opcode_for(op), [r])           # 2-byte instruction
    else:
        # At least one operand unknown — emit with feedback slot
        slot = allocate_slot()
        emit(opcode_for(op), [r, slot])     # 3-byte instruction
```

The function-level `FunctionTypeStatus` determines `CodeObject.immediate_jit_eligible`:

```python
code.type_status = result.env.function_status[fn.name]
code.immediate_jit_eligible = (code.type_status == FunctionTypeStatus.FULLY_TYPED)
```

---

## Python Package

The type checker lives in `code/packages/python/tetrad-type-checker/`.

Depends on `coding-adventures-tetrad-parser`.

### Public API

```python
from tetrad_type_checker import check, TypeCheckResult, TypeError, TypeWarning
from tetrad_type_checker.types import TypeInfo, FunctionTypeStatus, TypeEnvironment

# Type-check a parsed program.
# Never raises — errors are returned in TypeCheckResult.errors.
def check(program: Program) -> TypeCheckResult: ...

# Convenience: lex + parse + type-check in one call.
def check_source(source: str) -> TypeCheckResult: ...
```

---

## Test Strategy

### Type inference tests

- `fn f(a: u8, b: u8) -> u8 { return a + b; }` → all nodes inferred `u8`
- `fn g(a, b) { return a + b; }` → binary op inferred `Unknown`
- `fn h(a: u8, b) { return a + b; }` → binary op inferred `Unknown` (one unknown operand)
- `let x = 42;` → `x` type is `u8`
- `let x = in();` → `x` type is `Unknown`

### Function status tests

- All params + return typed, all body ops u8 → `FULLY_TYPED`
- No annotations → `UNTYPED`
- Some annotations → `PARTIALLY_TYPED`
- Typed function calling untyped function → `PARTIALLY_TYPED`

### Error tests

- `let x: u8 = in();` → `TypeError` (assigning Unknown to u8)
- `fn f() -> u8 { return in(); }` → `TypeError`
- `let x: foo = 1;` → `TypeError` (unknown type name)

### Warning tests

- Unannotated function → warning about JIT warmup
- Typed function calling untyped → warning about downgrade

### End-to-end tests

- Compile and execute: fully-typed `add` → `CodeObject.immediate_jit_eligible == True`
- Compile and execute: untyped `multiply` → `CodeObject.immediate_jit_eligible == False`
- Mixed program: typed and untyped functions coexist correctly

### Coverage target

95%+ line coverage.

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
