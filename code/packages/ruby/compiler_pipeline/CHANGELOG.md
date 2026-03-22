# Changelog

All notable changes to coding_adventures_compiler_pipeline will be documented here.

## [0.1.0] - 2026-03-18

### Added

- `Orchestrator` class with `run(source)` method that chains all stages
- Captures stage results as immutable Data objects:
  - `LexerStage` with tokens, token_count, source
  - `ParserStage` with ast, ast_dict (JSON-serializable)
  - `CompilerStage` with code, instructions_text, constants, names
  - `VMStage` with traces, final_variables, output
- `PipelineResult` bundling all stage outputs
- `ast_to_dict` helper for AST serialization
- `instruction_to_text` helper for human-readable bytecode display
- Support for optional keywords parameter
- Comprehensive test suite with >80% coverage
