# type-declarations

Language-agnostic type declaration format for the grammar pipeline — analogous
to TypeScript's `.d.ts` files.

## What it does

Defines the data structures that any parser emits as a side output alongside
its language-specific AST.  The `grammar-type-checker` consumes these
declarations to drive generic type inference over raw `GrammarASTNode` trees.

## Core types

| Type | Purpose |
|------|---------|
| `TypeDeclarations` | Container: named types + global bindings + typed mode |
| `KindDecl` | Base kind inferred for every expression |
| `AnnotatedNode` | `GrammarASTNode` + `KindDecl` at every node — the compilation artifact |
| `NamedTypeDecl` | Record, Union, or Alias declaration |

## KindDecl → IIR type_hint

Types are not ornamental — they feed AOT and JIT compilation directly:

```
KindDecl::Int            → type_hint = "i64"     (no boxing, native int ops)
KindDecl::Bool           → type_hint = "bool"    (direct branch)
KindDecl::Str            → type_hint = "str"     (string fast-path)
KindDecl::Function{n}    → type_hint = "closure" (direct closure dispatch)
everything else          → type_hint = "any"     (runtime profiling fallback)
```

The JIT and AOT specialisers prioritise `type_hint` over runtime profiles;
a fully-typed IIR module compiles to native code with zero warmup cost.

## Pipeline position

```
twig-parser::emit_type_declarations()
        ↓
TypeDeclarations  ─────────────────────────────────────────────┐
                                                                │
GrammarASTNode                                                  │
        ↓  grammar-type-checker::check(ast, decls, profile)    │
AnnotatedNode (KindDecl on every node)  ◄──────────────────────┘
        ↓  twig-ir-compiler::compile_annotated()
IIRModule (type_hint = iir_hint() on each instruction)
        ↓
JIT / AOT → typed native code
```

## Dependencies

None — pure data, zero dependencies.
