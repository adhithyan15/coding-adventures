# Changelog

## 0.1.0 — 2026-03-20

### Added
- Initial release
- GenericCompiler with pluggable rule handler registration
- Immutable functional design — handlers return updated compiler
- Instruction emission with emit/emit_jump/patch_jump
- Constant and name pool management with deduplication
- Scope tracking for local variables
- Pass-through for single-child AST nodes
- Nested CodeObject compilation
- CompilerError and UnhandledRuleError exceptions
