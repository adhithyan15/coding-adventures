# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial TypeScript port of the Python `coding-adventures-pipeline` package.
- `Pipeline` class that chains lexer, parser, compiler, and VM into a single execution flow.
- Stage capture interfaces: `LexerStage`, `ParserStage`, `CompilerStage`, `VMStage`.
- `PipelineResult` bundle containing all stage outputs.
- `astToDict()` function for converting AST nodes to JSON-serializable dictionaries.
- `instructionToText()` function for human-readable bytecode display.
- Self-contained `BytecodeCompiler` that compiles AST to stack-machine bytecode.
- Self-contained `VirtualMachine` that executes bytecode with trace capture.
- VM type definitions: `OpCode`, `Instruction`, `CodeObject`, `VMTrace`.
- Full literate programming comments throughout (Knuth-style).
- Comprehensive test suite with 7 test groups covering:
  - Basic pipeline end-to-end tests
  - Complex programs (precedence, parentheses, multiple statements, strings)
  - AST-to-dict conversion
  - Instruction-to-text conversion
  - Stage structure verification
  - BytecodeCompiler unit tests
  - VirtualMachine unit tests
