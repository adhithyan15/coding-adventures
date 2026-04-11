# SIR00 — Semantic IR

## Overview

The **Semantic IR (SIR)** is a language-agnostic Intermediate Representation for
programs. It sits between language-specific parsers (which produce concrete syntax
trees) and language-specific generators (which emit source code in a target language).

```
                        ┌─────────────────────────────────────┐
  JavaScript AST ───────►                                     ├──► JavaScript
  TypeScript AST ────────►         Semantic IR (SIR)          ├──► TypeScript
  Python AST ────────────►                                    ├──► Python
  Ruby AST ──────────────►                                    ├──► Go
  Go AST ────────────────►                                    ├──► Rust
                        └─────────────────────────────────────┘
```

Without a shared IR, every pair of (source language, target language) needs its own
compiler. With a shared IR, every front-end produces the SIR and every back-end
consumes it. N sources × M targets requires only N + M implementations.

This is the first package in the **SIR** (Semantic IR) series.

---

## Motivation: Why Not Use the Concrete Syntax Tree?

The concrete syntax tree (CST) from a grammar-driven parser mirrors the grammar's
structure exactly. A JavaScript `for-of` loop looks like:

```
ASTNode("for_of_statement",
  children=[
    Token("for"), Token("("), Token("let"), Token("x"), Token("of"),
    ASTNode("expression", ...), Token(")"),
    ASTNode("block", ...)
  ])
```

This is full of notation — the `for`, `(`, `let`, `)` tokens carry no semantic
meaning. The SIR strips all notation noise and keeps only what the program *means*:

```
SIRForOfStmt(binding="x", iterable=..., body=...)
```

The SIR also normalises across languages:

- A Python `for x in items:` and a JavaScript `for (const x of items)` produce
  the identical `SIRForOfStmt`.
- A TypeScript `const f: (n: number) => string` and a Python `def f(n: int) -> str`
  both produce `SIRFunctionDecl` with typed parameters and a typed return.

---

## Design Principles

### 1. Semantic, not syntactic

Strip all notation: brackets, semicolons, keywords-as-tokens, punctuation. Every
node represents a *meaning*, not a *notation*.

### 2. Immutable and typed

All nodes are immutable dataclasses (Python) or interfaces/classes (TypeScript).
Nodes are never mutated after construction; transformations produce new nodes.

### 3. Types as optional sidecars

Every expression node carries `resolved_type: SIRType` defaulting to `SIRAnyType()`.
Type inference enriches this field without changing node structure.

When source code has explicit type annotations (TypeScript, Rust, Java), they are
preserved in the SIR. When source code has no type annotations (JavaScript, Python
without hints), every expression starts as `SIRAnyType` and can be upgraded by a
separate inference pass.

### 4. Source location on every node

Every node carries `loc: SIRSourceLocation | None`. The lowering pass (AST → SIR)
populates this from the concrete syntax tree's position info. Transformations
preserve it. Emitters use it for error messages and future source map generation.

```
SIRSourceLocation:
  file: str | None    # source file path or identifier
  start_line: int     # 1-based line number
  start_col: int      # 0-based column number
  end_line: int       # 1-based line number
  end_col: int        # 0-based column number
```

### 5. Extension bags for language-specific metadata

Every node has an `extra: dict[str, Any]` field (defaults to empty). Language-specific
annotations that have no universal equivalent live here:

- Rust lifetime annotations: `extra["rust_lifetimes"] = ["'a", "'b"]`
- Python decorator list: `extra["python_decorators"] = [...]`
- JS `/*@__PURE__*/` hints: `extra["js_pure"] = True`
- TypeScript `readonly` modifier: `extra["ts_readonly"] = True`

The `extra` field is transparent to generators that don't understand it.

### 6. Escape hatch for untranslatable constructs

`SIRLangSpecific` wraps any construct that has no universal equivalent:

```
SIRLangSpecific(language="rust", construct="unsafe_block", children=[...])
SIRLangSpecific(language="python", construct="with_statement", children=[...])
```

Generators that understand the source language use it; others skip the children
or substitute a comment.

### 7. Desugar on entry

The `ast-to-sir` lowering pass normalises syntactic sugar to canonical forms before
it reaches the SIR:

- Python `@decorator\ndef f(): ...` → `SIRVariableDecl(name="f", value=SIRCall(callee=decorator, args=[f_fn]))`
- Python `[x*2 for x in xs if x > 0]` → `SIRForOfStmt` with conditional append
- `x += 1` → kept as `SIRAssignment(op="+=", ...)` (canonically supported)
- Multiple assignment `a = b = 1` → chain of `SIRAssignment` nodes

---

## Strictness Directionality

Languages occupy different positions on a strictness axis. Transformations through
the SIR are directional:

```
Rust   (ownership + exhaustive match)        ─── MOST STRICT
TypeScript strict (noImplicitAny: true)
TypeScript lenient (any allowed)
Python (with type hints)
JavaScript / Python (no hints)
Lua / Ruby / Perl (fully dynamic)            ─── LEAST STRICT
```

**Going down (projection):** Always mechanical and always correct. Java → Python
means dropping type annotations. No information needs to be invented.

**Going up (lifting):** Requires inventing information that does not exist. Python →
Java means deciding what type every variable has. This can only be approximated by
inference. The result may be `Object` everywhere (valid but useless) or type-inferred
(useful but fallible).

Concretely:

```
TypeScript → JavaScript  ✅  strip types, always correct
JavaScript → TypeScript  ⚠️  requires inference; starts as any everywhere
Java → Python            ✅  drop types and access modifiers
Python → Java            ⚠️  requires inference; Object/any everywhere without it
Python → Rust            ❌  ownership inference is unsolved; out of scope
```

### Emitter modes

| Mode | Behaviour when type is SIRAnyType |
|------|-----------------------------------|
| `STRICT` | Fail — refuse to emit without type coverage |
| `LENIENT` | Emit `any` / `Object` (valid output, low quality) |
| `INFERRED` | Run type inference first; fall back to `any` where it fails |

The `resolved_type` field on every expression carries what is known. Emitters
consult it and apply the chosen mode.

---

## Node Reference

### Source Location

```
SIRSourceLocation
  file: str | None      source file path or name; None if unknown
  start_line: int       1-based
  start_col: int        0-based
  end_line: int         1-based
  end_col: int          0-based
```

---

### Type Nodes

Every expression carries `resolved_type: SIRType`. The default is `SIRAnyType()`.

```
SIRAnyType                       unknown / untyped (the default)
SIRNeverType                     unreachable / bottom type
SIRVoidType                      no return value (void functions)

SIRPrimitiveType(name)
  name: "string" | "number" | "boolean" | "null" | "undefined"
      | "symbol" | "bigint"

SIRUnionType(types)              T | U | V
SIRIntersectionType(types)       T & U (TypeScript intersection)
SIRArrayType(element)            T[]
SIRTupleType(elements)           [T, U, V]

SIRObjectType(fields)
  fields: list of SIRObjectField(name, value_type, required)

SIRFunctionType(params, return_type)

SIRGenericType(name, args)       Array<T>, Map<K, V>, Promise<T>

SIRReferenceType(name)           unresolved named type "MyClass"
                                 resolved by a type-checker pass
```

---

### Module Root

```
SIRModule
  source_language: str | None    "javascript", "typescript", "python", …
  body: list of (SIRDeclaration | SIRStatement)
  loc: SIRSourceLocation | None
  extra: dict
```

---

### Declarations

```
SIRVariableDecl
  name: str
  kind: "let" | "const" | "var" | "assign"   "assign" = Python / Ruby style
  value: SIRExpression | None
  type_annotation: SIRType                    SIRAnyType if not annotated
  loc: SIRSourceLocation | None
  extra: dict

SIRFunctionDecl
  name: str
  params: list of SIRParam
  return_type: SIRType
  body: SIRBlock
  is_async: bool
  is_generator: bool
  loc: SIRSourceLocation | None
  extra: dict

SIRClassDecl
  name: str
  superclass: str | None
  type_params: list of str          generic type parameter names
  members: list of (SIRMethodDef | SIRPropertyDef)
  loc: SIRSourceLocation | None
  extra: dict

SIRMethodDef
  name: str
  kind: "constructor" | "method" | "getter" | "setter" | "static_method"
  params: list of SIRParam
  return_type: SIRType
  body: SIRBlock
  loc: SIRSourceLocation | None
  extra: dict

SIRPropertyDef
  name: str
  value: SIRExpression | None
  type_annotation: SIRType
  static: bool
  loc: SIRSourceLocation | None
  extra: dict

# TypeScript / Java / C# specific — carried through when source has them

SIRInterfaceDecl
  name: str
  extends: list of str
  members: list of (SIRPropertySignature | SIRMethodSignature)
  loc: SIRSourceLocation | None
  extra: dict

SIRTypeAliasDecl
  name: str
  value: SIRType
  loc: SIRSourceLocation | None
  extra: dict

SIRImport
  source: str                  module path string
  default: str | None          import Foo from "..."
  namespace: str | None        import * as ns from "..."
  specifiers: list of SIRImportSpecifier(imported, local)
  loc: SIRSourceLocation | None
  extra: dict

SIRExport
  default: SIRExpression | SIRDeclaration | None
  specifiers: list of SIRExportSpecifier(local, exported)
  source: str | None           re-export: export { x } from "..."
  loc: SIRSourceLocation | None
  extra: dict
```

---

### Parameters

```
SIRParam
  name: str
  type_annotation: SIRType    SIRAnyType if not annotated
  default_value: SIRExpression | None
  rest: bool                  *args / ...rest spread parameter
  loc: SIRSourceLocation | None
  extra: dict
```

---

### Statements

```
SIRBlock
  body: list of (SIRDeclaration | SIRStatement)
  loc: SIRSourceLocation | None

SIRExpressionStmt
  expression: SIRExpression
  loc: SIRSourceLocation | None

SIRIfStmt
  test: SIRExpression
  consequent: SIRBlock
  alternate: SIRBlock | None
  loc: SIRSourceLocation | None

SIRWhileStmt
  test: SIRExpression
  body: SIRBlock
  loc: SIRSourceLocation | None

SIRForOfStmt          JS for-of / Python for-in (iterates values)
  binding: str
  iterable: SIRExpression
  body: SIRBlock
  is_await: bool        for await (const x of gen)
  loc: SIRSourceLocation | None

SIRForInStmt          JS for-in (iterates keys)
  binding: str
  object: SIRExpression
  body: SIRBlock
  loc: SIRSourceLocation | None

SIRForStmt            C-style: for (init; test; update)
  init: SIRVariableDecl | SIRExpression | None
  test: SIRExpression | None
  update: SIRExpression | None
  body: SIRBlock
  loc: SIRSourceLocation | None

SIRReturnStmt
  value: SIRExpression | None
  loc: SIRSourceLocation | None

SIRThrowStmt
  value: SIRExpression
  loc: SIRSourceLocation | None

SIRTryStmt
  body: SIRBlock
  handler: SIRCatchClause | None
  finalizer: SIRBlock | None
  loc: SIRSourceLocation | None

SIRCatchClause
  binding: str | None           the variable name in catch (e)
  type_annotation: SIRType
  body: SIRBlock
  loc: SIRSourceLocation | None

SIRBreakStmt
  label: str | None
  loc: SIRSourceLocation | None

SIRContinueStmt
  label: str | None
  loc: SIRSourceLocation | None

SIRSwitchStmt
  discriminant: SIRExpression
  cases: list of SIRSwitchCase
  loc: SIRSourceLocation | None

SIRSwitchCase
  test: SIRExpression | None    None = default:
  body: list of SIRStatement
  loc: SIRSourceLocation | None

SIRLangSpecific                 escape hatch for untranslatable constructs
  language: str                 "python", "rust", "elixir", …
  construct: str                "with_statement", "unsafe_block", …
  children: list of SIRNode     traversable sub-nodes
  loc: SIRSourceLocation | None
  extra: dict
```

---

### Expressions

Every expression node carries `resolved_type: SIRType` (default `SIRAnyType()`)
and `loc: SIRSourceLocation | None`.

```
SIRLiteral
  value: int | float | str | bool | None
  resolved_type: SIRType
  loc: SIRSourceLocation | None
  extra: dict

SIRIdentifier
  name: str
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRBinaryOp
  op: str     "+", "-", "*", "/", "%", "**",
              "===", "!==", "==", "!=", "<", ">", "<=", ">=",
              "&&", "||", "??", "&", "|", "^", "<<", ">>", ">>>"
  left: SIRExpression
  right: SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRUnaryOp
  op: str     "-", "+", "!", "~", "typeof", "void", "delete",
              "not" (Python), "await", "yield" (as prefix ops)
  operand: SIRExpression
  prefix: bool
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRAssignment
  op: str     "=", "+=", "-=", "*=", "/=", "%=", "**=",
              "&&=", "||=", "??=", "&=", "|=", "^=",
              "<<=", ">>=", ">>>="
  target: SIRExpression     SIRIdentifier | SIRMemberAccess | SIRIndex
  value: SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRCall
  callee: SIRExpression
  args: list of (SIRExpression | SIRSpread)
  type_args: list of SIRType    TypeScript generic type arguments
  is_new: bool                  new Foo() vs Foo()
  is_optional: bool             foo?.()
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRMemberAccess
  object: SIRExpression
  property: str
  computed: bool    obj[prop] (True) vs obj.prop (False)
  optional: bool    obj?.prop
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRIndex
  object: SIRExpression
  index: SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRConditional                  ternary: test ? consequent : alternate
  test: SIRExpression
  consequent: SIRExpression
  alternate: SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRObjectLiteral
  properties: list of (SIRProperty | SIRSpread)
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRProperty
  key: str | SIRExpression    str = static key; SIRExpression = computed
  value: SIRExpression
  shorthand: bool             {x} shorthand for {x: x}
  computed: bool              {[expr]: val}

SIRArrayLiteral
  elements: list of (SIRExpression | SIRSpread | None)    None = hole
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRSpread
  value: SIRExpression

SIRArrowFunction
  params: list of SIRParam
  body: SIRBlock | SIRExpression    block body or concise expression body
  return_type: SIRType
  is_async: bool
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRFunctionExpression
  name: str | None
  params: list of SIRParam
  body: SIRBlock
  return_type: SIRType
  is_async: bool
  is_generator: bool
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRTemplateLiteral              `hello ${name}`
  quasis: list of str           the string parts (len = len(expressions) + 1)
  expressions: list of SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRAwait
  value: SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRYield
  value: SIRExpression | None
  delegate: bool                yield* (delegates to another iterable)
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRTypeAssertion                TypeScript: expr as Type
  value: SIRExpression
  target_type: SIRType
  resolved_type: SIRType
  loc: SIRSourceLocation | None

SIRSequence                     comma operator: (a, b, c)
  expressions: list of SIRExpression
  resolved_type: SIRType
  loc: SIRSourceLocation | None
```

---

## Worked Example

### Input (JavaScript)

```javascript
function greet(name) {
  const msg = "Hello, " + name;
  return msg;
}
```

### SIR

```
SIRModule(source_language="javascript", body=[
  SIRFunctionDecl(
    name="greet",
    params=[SIRParam(name="name", type_annotation=SIRAnyType())],
    return_type=SIRAnyType(),
    body=SIRBlock(body=[
      SIRVariableDecl(
        name="msg",
        kind="const",
        value=SIRBinaryOp(
          op="+",
          left=SIRLiteral(value="Hello, "),
          right=SIRIdentifier(name="name")
        )
      ),
      SIRReturnStmt(value=SIRIdentifier(name="msg"))
    ])
  )
])
```

### Back to JavaScript (via sir-to-js-ast + js-ast-to-string)

```javascript
function greet(name) {
  const msg = "Hello, " + name;
  return msg;
}
```

Round-trip preserves semantics exactly. Formatting may change (consistent indentation,
quote style), but the program is identical in meaning.

### Back to TypeScript (LENIENT mode)

```typescript
function greet(name: any): any {
  const msg: any = "Hello, " + name;
  return msg;
}
```

### Back to TypeScript (INFERRED mode, after type inference)

```typescript
function greet(name: string): string {
  const msg: string = "Hello, " + name;
  return msg;
}
```

---

## Package Matrix

| Language | Package Directory | Module / Namespace |
|----------|-------------------|--------------------|
| Python | `code/packages/python/semantic-ir/` | `coding_adventures_semantic_ir` |
| TypeScript | `code/packages/typescript/semantic-ir/` | `@coding-adventures/semantic-ir` |
| Go | `code/packages/go/semantic-ir/` | `semanticir` |
| Rust | `code/packages/rust/semantic-ir/` | `semantic_ir` |
| Ruby | `code/packages/ruby/semantic_ir/` | `CodingAdventures::SemanticIR` |
| Elixir | `code/packages/elixir/semantic_ir/` | `CodingAdventures.SemanticIR` |
| Lua | `code/packages/lua/semantic_ir/` | `coding_adventures.semantic_ir` |
| Perl | `code/packages/perl/semantic-ir/` | `CodingAdventures::SemanticIR` |
| Swift | `code/packages/swift/semantic-ir/` | `SemanticIR` |

---

## Pipeline Packages

The SIR is a shared foundation. Dependent packages form the full
source-to-source pipeline:

```
JS Source
  ─► [javascript-lexer v=es2025]    tokens
  ─► [javascript-parser v=es2025]   ASTNode tree
  ─► [js-ast-to-sir]                SIRModule (all types = SIRAnyType)
  ─► [sir-to-js-ast]                ASTNode tree (canonical)
  ─► [js-ast-to-string]             formatted JS string
```

Later additions:

```
  ─► [type-inference-sidecar]       enriches SIRAnyType → concrete types
  ─► [sir-to-ts-ast]                TypeScript ASTNode tree with type annotations
  ─► [js-prettier-rules]            normalise formatting on JS AST
```

---

## SIR Series Roadmap

| Spec | Package | Concepts |
|------|---------|----------|
| SIR00 | `semantic-ir` | Node taxonomy, type sidecar, source location sidecar |
| SIR01 | `js-ast-to-sir` | Lowering pass: ES2025 ASTNode → SIRModule |
| SIR02 | `sir-to-js-ast` | Lifting pass: SIRModule → JS ASTNode |
| SIR03 | `js-ast-to-string` | Unparser: JS ASTNode → source string |
| SIR04 | `js-pipeline` | End-to-end: JS source → lex → parse → SIR → AST → string |
| SIR05 | `sir-type-inference` | Forward-propagation type inference (Flow-style) |
| SIR06 | `sir-to-ts-ast` | TypeScript generator: SIRModule → TS ASTNode |
| SIR07 | `py-ast-to-sir` | Python front-end: Python ASTNode → SIRModule |
