# Changelog

All notable changes to the pipeline-visualizer package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added
- TypeScript port of the pipeline-visualizer package (formerly html-renderer)
- `HTMLRenderer` class with `render()`, `renderFromJson()`, and `renderToFile()` methods
- Full PipelineReport type definitions matching the JSON data contract
- Stage renderers for all pipeline stages:
  - Lexer: colored token badges with type classification
  - Parser: SVG tree diagram with recursive layout algorithm
  - Compiler: bytecode instruction table with constants/names pools
  - VM: step-by-step execution trace with stack visualization
  - Assembler: assembly listing with color-coded binary encoding
  - RISC-V/ARM: register state trace with change highlighting
  - ALU: bit-level arithmetic display with CPU flags
  - Gates: gate-level circuit trace grouped by operation
- Fallback renderer for unknown stage types (formatted JSON)
- HTML escaping for XSS prevention
- Self-contained HTML output with embedded CSS (Catppuccin Mocha theme)
- Comprehensive test suite covering all renderers, types, and edge cases
