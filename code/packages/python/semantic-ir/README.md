# coding-adventures-semantic-ir

Language-agnostic Semantic IR (SIR) — the pivot point for cross-language compilation.

## What is this?

The Semantic IR sits between language-specific parsers (which produce concrete syntax
trees) and language-specific generators (which emit source code). It is a normalized,
language-neutral representation of a program's *meaning*, not its *notation*.

```
JS source → [js-lexer] → tokens → [js-parser] → ASTNode
  → [js-ast-to-sir] → SIRModule    ← this package defines SIRModule
  → [sir-to-js-ast] → ASTNode
  → [js-ast-to-string] → JS output
```

The same `SIRModule` can be fed to a TypeScript generator, a Python generator, or any
other backend — that is the whole point.

## Design principles

1. **Semantic, not syntactic** — tokens, delimiters, and keywords are stripped. A JS
   `for-of` loop and a Python `for-in` loop both become `SIRForOfStmt`.

2. **Immutable and typed** — all nodes are `@dataclass` instances. Treat them as
   immutable: produce new nodes instead of mutating.

3. **Types as optional sidecars** — every expression carries `resolved_type: SIRType`
   defaulting to `SIRAnyType()`. Type inference enriches this field in place without
   changing node structure.

4. **Source location on every node** — `loc: SIRSourceLocation | None` is present on
   every node. Populated by the lowering pass from the parser's position info.

5. **Extension bags** — every node has `extra: dict` for language-specific metadata
   that has no universal equivalent (Rust lifetimes, Python decorators, JS `@__PURE__`).

6. **Escape hatch** — `SIRLangSpecific` wraps constructs that cannot be normalised
   (`unsafe {}` in Rust, `with` statements in Python).

## Strictness directionality

Going *down* the strictness ladder (TypeScript → JavaScript, Java → Python) is always
mechanical: drop the information the target doesn't need.

Going *up* (JavaScript → TypeScript, Python → Java) requires inventing information that
doesn't exist in the source. Without inference, every unknown type becomes `SIRAnyType`,
which generators emit as `any` (TypeScript) or `Object` (Java).

The three emitter modes:

| Mode | Behaviour |
|------|-----------|
| `STRICT` | Fail if any expression's `resolved_type` is `SIRAnyType` |
| `LENIENT` | Emit `any` / `Object` wherever `resolved_type` is `SIRAnyType` |
| `INFERRED` | Run type-inference pass first; fall back to `any` for unknowns |

## Node taxonomy

### Type nodes (the sidecar)

| Node | Meaning |
|------|---------|
| `SIRAnyType` | Unknown / untyped (default) |
| `SIRNeverType` | Bottom type — unreachable |
| `SIRVoidType` | No return value |
| `SIRPrimitiveType` | `string`, `number`, `boolean`, `null`, `undefined`, `symbol`, `bigint` |
| `SIRUnionType` | `T \| U` |
| `SIRIntersectionType` | `T & U` |
| `SIRArrayType` | `T[]` |
| `SIRObjectType` | `{ key: T }` structural shape |
| `SIRFunctionType` | `(T, U) => V` |
| `SIRGenericType` | `Array<T>`, `Map<K, V>` |
| `SIRReferenceType` | Unresolved named type |
| `SIRTupleType` | `[T, U, V]` |

### Declarations

`SIRVariableDecl`, `SIRFunctionDecl`, `SIRClassDecl`, `SIRInterfaceDecl`,
`SIRTypeAliasDecl`, `SIRImport`, `SIRExport`.

### Statements

`SIRBlock`, `SIRExpressionStmt`, `SIRIfStmt`, `SIRWhileStmt`, `SIRForOfStmt`,
`SIRForInStmt`, `SIRForStmt`, `SIRReturnStmt`, `SIRThrowStmt`, `SIRTryStmt`,
`SIRBreakStmt`, `SIRContinueStmt`, `SIRSwitchStmt`, `SIRLangSpecific`.

### Expressions

`SIRLiteral`, `SIRIdentifier`, `SIRBinaryOp`, `SIRUnaryOp`, `SIRAssignment`,
`SIRCall`, `SIRMemberAccess`, `SIRIndex`, `SIRConditional`, `SIRObjectLiteral`,
`SIRArrayLiteral`, `SIRSpread`, `SIRArrowFunction`, `SIRFunctionExpression`,
`SIRTemplateLiteral`, `SIRAwait`, `SIRYield`, `SIRTypeAssertion`, `SIRSequence`.

## Usage example

```python
from coding_adventures_semantic_ir import (
    SIRModule, SIRFunctionDecl, SIRParam, SIRBlock,
    SIRReturnStmt, SIRBinaryOp, SIRLiteral, SIRIdentifier,
    SIRPrimitiveType, SIRSourceLocation,
)

# Represent: function greet(name) { return "Hello, " + name; }
module = SIRModule(
    source_language="javascript",
    body=[
        SIRFunctionDecl(
            name="greet",
            params=[SIRParam(name="name")],
            return_type=SIRPrimitiveType("string"),
            body=SIRBlock(body=[
                SIRReturnStmt(value=SIRBinaryOp(
                    op="+",
                    left=SIRLiteral(value="Hello, "),
                    right=SIRIdentifier(name="name"),
                ))
            ]),
        )
    ],
)

assert module.type == "module"
fn = module.body[0]
assert fn.name == "greet"
```

## Installation

```bash
pip install coding-adventures-semantic-ir
```

Or in development mode:

```bash
uv venv
uv pip install -e ".[dev]"
```

## Running tests

```bash
.venv/bin/python -m pytest tests/ -v
```

## Related packages

| Package | Role |
|---------|------|
| `coding-adventures-javascript-lexer` | JS source → tokens |
| `coding-adventures-javascript-parser` | Tokens → ASTNode |
| `coding-adventures-js-ast-to-sir` | ASTNode → SIRModule (lowering) |
| `coding-adventures-sir-to-js-ast` | SIRModule → ASTNode (lifting) |
| `coding-adventures-js-ast-to-string` | ASTNode → JS string |

## Specification

See `code/specs/SIR00-semantic-ir.md` in the repository.
