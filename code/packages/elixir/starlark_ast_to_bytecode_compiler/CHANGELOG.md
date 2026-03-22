# Changelog

## 0.1.1 — 2026-03-22

### Fixed
- Added INDENT/DEDENT token injection to tokenizer for proper block parsing
- Multi-statement function bodies and if-blocks now parse correctly
- Multiline function calls (with NEWLINE/INDENT/DEDENT inside parens) no longer hang
- `handle_def_stmt` now correctly captures the BUILD_TUPLE emission for default args
- Added `skip_whitespace` helper that skips NEWLINE/INDENT/DEDENT in non-block contexts

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
