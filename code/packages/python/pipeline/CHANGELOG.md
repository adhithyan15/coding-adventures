# Changelog

All notable changes to the pipeline package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added
- Initial package scaffolding with pyproject.toml, src layout, and test structure
- Pipeline orchestrator (`orchestrator.py`) that chains Lexer -> Parser -> Compiler -> VM
- Stage dataclasses: `LexerStage`, `ParserStage`, `CompilerStage`, `VMStage`, `PipelineResult`
- `ast_to_dict()` helper for JSON-serializable AST representation (for HTML visualizer)
- `instruction_to_text()` helper for human-readable bytecode display
- `Pipeline` class with `run()` method as the main entry point
- 40 tests across 5 test groups with 100% code coverage
- Knuth-style literate documentation throughout
