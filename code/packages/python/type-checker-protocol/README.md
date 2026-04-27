# coding-adventures-type-checker-protocol

Generic type-checker protocol and framework for the coding-adventures compiler stack.

This package defines the **shared interface** that all language type checkers
in this repository must implement, and a small reusable framework for
building AST-driven checkers on top of that interface. Future languages —
Nib, TypeScript, C, and others — can ship their own type checker while still
plugging cleanly into the same compiler pipeline.

---

## What Is a Type Checker?

A compiler typically transforms source text through a series of stages:

```
Source text
  → Lexer             (characters → tokens)
  → Parser            (tokens → untyped AST)
  → Type Checker      (untyped AST → typed AST)   ← this layer (language semantics)
  → IR Compiler       (typed AST → IR)
  → Backend Validator (IR → validated IR)          ← ISA/hardware checks live here
  → Code Generator    (validated IR → machine code / bytecode)
```

The **type checker** sits between the parser and the IR compiler.  Its two
jobs are:

1. **Verify** that the program is type-safe — catch mistakes like adding a
   number to a string before they become mysterious runtime crashes.
2. **Annotate** the AST with type information — stamp each node with its
   resolved type so that later stages (IR generation, optimization) don't
   need to re-derive it.

### What TypeChecker does NOT do

`TypeChecker` enforces **language-level invariants only**.  It must not
enforce hardware or ISA constraints.  For example:

| Belongs in `TypeChecker`           | Belongs in the backend validator        |
|------------------------------------|-----------------------------------------|
| Type mismatch (`Int + String`)     | Call depth ≤ 2 (Intel 4004 limit)       |
| Undeclared variable                | Total RAM ≤ 160 bytes (4004 limit)      |
| Language-level no-recursion rule   | Register count ≤ 16 (architecture rule) |
| Static for-loop bounds (language)  | Instruction encoding constraints        |

Hardware and ISA constraints belong in each backend's own validator (e.g.,
the `IrValidator` in the `intel-4004-ir-validator` package), which runs *after* IR
generation.  This separation keeps the design **composable**: the same Nib
type checker works unchanged whether you target Intel 4004, ARM, WASM, or any
future ISA.

For example, given the Nib expression `1 + "hello"`, the type checker would:

- Walk the `+` node.
- Infer the left operand `1` has type `Int`.
- Infer the right operand `"hello"` has type `String`.
- Detect the mismatch and produce a `TypeErrorDiagnostic` at the right line
  and column.
- Return the (partially annotated) AST together with the error list so the
  user can see *all* the errors in one go, not just the first one.

---

## Why a Protocol / Interface?

This repo will have **many** type checkers:

| Package                     | What it checks         |
|-----------------------------|------------------------|
| `nib-type-checker`          | The Nib language       |
| `lattice-type-checker`      | Lattice stylesheets    |
| `mosaic-type-checker`       | Mosaic components      |
| … more to come …            |                        |

All of them must implement the **same interface** so that:

- The compiler pipeline can compose them uniformly.
- Each checker can be unit-tested in isolation against the same contract.
- Tools (dashboards, linters, IDEs) can treat all type checkers the same way.

Python's `typing.Protocol` gives us **structural subtyping** — a class
satisfies the protocol if it has the right methods with the right signatures,
without needing to inherit from anything.  This is sometimes called
*duck typing with types*: if it walks like a type checker and quacks like a
type checker, it *is* a type checker — mypy will verify this statically.

---

## Core Types

### `TypeErrorDiagnostic`

A frozen (immutable) dataclass representing a single type error:

```python
@dataclass(frozen=True)
class TypeErrorDiagnostic:
    message: str   # human-readable description
    line: int      # 1-based line number in the source
    column: int    # 1-based column number within the line
```

**Frozen** means you cannot accidentally mutate a diagnostic after creating
it.  It also makes diagnostics hashable, so you can put them in sets.

```python
from type_checker_protocol import TypeErrorDiagnostic

err = TypeErrorDiagnostic(
    message="Cannot add Int and String",
    line=5,
    column=12,
)
print(err)
# TypeErrorDiagnostic(message='Cannot add Int and String', line=5, column=12)
```

---

### `TypeCheckResult[ASTOut]`

A frozen dataclass wrapping the result of a complete type-checking pass:

```python
@dataclass(frozen=True)
class TypeCheckResult(Generic[ASTOut]):
    typed_ast: ASTOut
    errors: list[TypeErrorDiagnostic]

    @property
    def ok(self) -> bool:
        return len(self.errors) == 0
```

Key design decisions:

- **Always returns a result, never raises.**  Exceptions are for unexpected
  internal errors, not for type errors the programmer made.
- **Collects all errors in one pass.**  The user sees every problem at once,
  not just the first one.
- **Carries the AST even on failure.**  IDEs can use the partially-annotated
  AST for completions and hover types while the user is still fixing mistakes.
- **`.ok` shorthand.**  The most common check (`if result.ok`) reads naturally.

```python
from type_checker_protocol import TypeCheckResult, TypeErrorDiagnostic

# Success
result: TypeCheckResult[MyTypedAST] = TypeCheckResult(
    typed_ast=my_typed_ast,
    errors=[],
)
assert result.ok  # True

# Failure
err = TypeErrorDiagnostic("Bad type", line=3, column=7)
result = TypeCheckResult(typed_ast=partial_ast, errors=[err])
assert not result.ok  # False
assert result.errors[0].line == 3
```

---

### `TypeChecker[ASTIn, ASTOut]`

The protocol every type checker must satisfy:

```python
class TypeChecker(Protocol[ASTIn, ASTOut]):
    def check(self, ast: ASTIn) -> TypeCheckResult[ASTOut]:
        ...
```

**Why generics?**  Different languages have different AST types.  A Nib type
checker works on `NibNode` objects; a Lattice type checker works on
`LatticeNode` objects.  The two type parameters let mypy enforce that you
don't accidentally pass the wrong AST to the wrong checker:

```python
# mypy catches this at compile time — wrong checker for the wrong AST:
nib_checker: TypeChecker[NibNode, TypedNibNode] = LatticeTypeChecker()  # type error!
```

**Structural typing — no inheritance needed.**  A class automatically
satisfies the protocol as long as it has the right `check` method.  You never
need to write `class MyChecker(TypeChecker[...])`.

### `GenericTypeChecker[AST]`

For projects that want more than a bare protocol, the package also exports a
small framework class:

- owns the standard `check(ast) -> TypeCheckResult[ast]` lifecycle
- accumulates diagnostics through a shared `_error(...)` helper
- dispatches full AST nodes to language-specific handlers or hooks

That makes it practical to build a Nib checker today and a TypeScript or C
checker later without rewriting the common orchestration each time.

---

## Usage Example

```python
from dataclasses import dataclass
from type_checker_protocol import TypeChecker, TypeCheckResult, TypeErrorDiagnostic


# --- AST node types (defined in your language package) ---

@dataclass
class NibNode:
    kind: str
    children: list["NibNode"]

@dataclass
class TypedNibNode:
    kind: str
    children: list["TypedNibNode"]
    resolved_type: str


# --- Concrete type checker (no inheritance needed) ---

class NibTypeChecker:
    """Type-checks Nib ASTs."""

    def check(self, ast: NibNode) -> TypeCheckResult[TypedNibNode]:
        errors: list[TypeErrorDiagnostic] = []
        typed = self._check_node(ast, errors)
        return TypeCheckResult(typed_ast=typed, errors=errors)

    def _check_node(
        self,
        node: NibNode,
        errors: list[TypeErrorDiagnostic],
    ) -> TypedNibNode:
        if node.kind == "add":
            # … infer types of children, check compatibility …
            pass
        return TypedNibNode(kind=node.kind, children=[], resolved_type="int")


# --- Using the protocol annotation ---

def compile_nib(
    checker: TypeChecker[NibNode, TypedNibNode],
    ast: NibNode,
) -> TypedNibNode:
    result = checker.check(ast)
    if not result.ok:
        for err in result.errors:
            print(f"  {err.line}:{err.column}: {err.message}")
        raise SystemExit("Type errors found.")
    return result.typed_ast


checker = NibTypeChecker()
typed_ast = compile_nib(checker, my_ast)
```

---

## How This Fits in the Compiler Pipeline

```
coding-adventures-type-checker-protocol   ← you are here (language semantics only)
        ↑ implemented by
coding-adventures-nib-type-checker
coding-adventures-lattice-type-checker
… etc.
        ↓ produces TypeCheckResult[TypedAST]
coding-adventures-compiler-ir             (IR generation)
        ↓ produces IR
intel-4004-ir-validator IrValidator       (ISA/hardware constraint checks)
ir-to-intel-4004-compiler                 (Intel 4004 assembly generation)
arm-backend IrValidator                   (different ISA, same Nib frontend)
        ↓
coding-adventures-bytecode-compiler / machine code
```

The protocol lives at the **bottom** of the type-checking layer.  It has zero
dependencies (only Python stdlib).  Every concrete type checker in the repo
depends on it; nothing below it does.

Because hardware constraints are enforced separately in each backend validator,
the **same frontend type checker** can target any ISA.  Adding a new backend
never requires touching `TypeChecker` or any language-level checker.

---

## Installation

```bash
pip install coding-adventures-type-checker-protocol
```

For development:

```bash
pip install "coding-adventures-type-checker-protocol[dev]"
pytest
```

---

## License

MIT
