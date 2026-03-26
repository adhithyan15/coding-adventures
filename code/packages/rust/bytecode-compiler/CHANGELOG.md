# Changelog

## 0.1.0 — 2026-03-20

### Added
- `GenericCompiler` — pluggable AST-to-bytecode compiler with rule handler registration
- `ASTNode`, `ASTChild`, `TokenNode` — AST types for grammar-produced trees
- `CompilerScope` — scope management for compiling nested function bodies
- Instruction emission: `emit`, `emit_jump`, `patch_jump`
- Constants and names management with deduplication
- Scope stack for nested compilation (`enter_scope`, `exit_scope`, `compile_nested`)
- Default pass-through behavior for unregistered AST rules
- `compile()` entry point with optional halt opcode
