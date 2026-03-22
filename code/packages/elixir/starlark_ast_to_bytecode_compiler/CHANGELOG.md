# Changelog

## 0.1.0 — 2026-03-22

### Added
- All 46 Starlark opcodes as module functions in `Opcodes`
- Operator-to-opcode mappings: binary, comparison, augmented assignment, unary
- ~55 grammar rule handlers covering all Starlark language constructs
- Self-contained tokenizer and parser for standalone compilation
- `compile_starlark/1` convenience function for source-to-bytecode
- `compile_ast/1` for compiling pre-parsed ASTs
- `create_compiler/0` factory for configured GenericCompiler
- 80+ unit tests covering opcodes, compilation, and code generation
- BUILD file, README.md, CHANGELOG.md
