# grammar-type-checker

Generic type checker operating on `GrammarASTNode` trees with parser-emitted
`TypeDeclarations`.  Returns a fully-annotated `AnnotatedNode` tree that feeds
AOT/JIT compilation via IIR `type_hint` fields.

## What it does

Walks a raw `GrammarASTNode` tree (from the `parser` crate) using:

1. **`TypeDeclarations`** — parser-emitted type rules (like TypeScript `.d.ts`):
   named types (records, unions, aliases), global binding kinds, and the
   module's typed-mode setting.

2. **`LanguageProfile`** — language-specific tree navigation: which grammar
   rule names correspond to literals, variable references, function calls, etc.

Returns `TypeCheckResult<AnnotatedNode>` where every node in the tree carries
its inferred `KindDecl`.

## Types feed compilation

The `AnnotatedNode.iir_hint()` method maps inferred kinds to IIR `type_hint`
strings:

```
KindDecl::Int  → "i64"      (64-bit int ops, no boxing)
KindDecl::Bool → "bool"     (direct branch, no type guard)
KindDecl::Str  → "str"      (string fast-path)
Function{n}    → "closure"  (direct dispatch)
others         → "any"      (fall back to runtime profiling)
```

JIT and AOT specialisers prioritise `type_hint` over runtime profiles; a
fully-typed IIR function compiles to native code with zero warmup cost.

## Implementing LanguageProfile

```rust
use grammar_type_checker::{LanguageProfile, AppInfo, BinderInfo};
use parser::grammar_parser::GrammarASTNode;
use type_declarations::KindDecl;

struct MyLangProfile;

impl LanguageProfile for MyLangProfile {
    fn literal_kind(&self, node: &GrammarASTNode) -> Option<KindDecl> {
        match node.rule_name.as_str() {
            "int_literal" => Some(KindDecl::Int),
            "bool_literal" => Some(KindDecl::Bool),
            _ => None,
        }
    }
    // ... implement remaining methods
}
```

## Checks performed

| Check | When emitted |
|-------|-------------|
| Unresolved variable | `VarRef` not in scope or globals |
| Call arity mismatch | `Apply` where callee is `Function{n}` and args ≠ n |
| Non-exhaustive match | `Match` on a union with uncovered variants |

Checks are **mode-gated**:

| Mode | Behaviour |
|------|-----------|
| `Off` | No errors emitted; annotated tree still built |
| `Lenient` | Errors collected; `ok: true` always |
| `Strict` | Errors collected; `ok: false` if any errors |

## Dependencies

- `parser` — `GrammarASTNode`, `ASTNodeOrToken`
- `type-declarations` — `TypeDeclarations`, `KindDecl`, `AnnotatedNode`
- `type-checker-protocol` — `TypeCheckResult`, `TypeErrorDiagnostic`
