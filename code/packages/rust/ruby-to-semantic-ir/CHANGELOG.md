# Changelog

All notable changes to the `ruby-to-semantic-ir` crate will be documented in this file.

## [0.1.0] - 2026-05-20

### Added (Phase 5 — initial Ruby → SIR frontend)
- New crate `ruby-to-semantic-ir`.  Consumes `coding-adventures-ruby-parser`'s `GrammarASTNode` and emits a `semantic_ir::Module`.
- `compile_source(source, module_name) -> Result<Module, RubyLowerError>` — tokenize → parse → lower in one call.
- `compile(ast, module_name) -> Result<Module, RubyLowerError>` — lower an already-parsed AST.
- `RubyLowerError` — carries a human-readable message plus 1-based line/column drawn from the AST node's span.
- v0 lowering covers everything ruby-parser v0 parses:
  - Programs → synthesised `main` function whose body is the sequence of lowered statements.  Last bare-expression statement becomes the block's `value`; otherwise `value = NilLit`.
  - Assignments (`x = expr`) → `LetBinding` for first occurrence, `Assign` for subsequent re-bindings (scope is `Local`).
  - Method calls (`name(args...)`) → `BuiltinCall` for the known builtin set (`puts`, `print`, `p`, `gets`, `raise`); other names lower to `DirectCall` (placeholder; in v0 there are no user-defined functions, so backends will flag these as unresolved).
  - Expressions: integer literals, string literals, name references, binary `+ - * /` lowered to `BuiltinCall("+"...)` etc., parenthesised sub-expressions.
- Tests cover empty program, single assignment, multi-statement programs, arithmetic, method calls, re-assignment routing through `Stmt::Assign`, and tail-expression promotion to block `value`.
