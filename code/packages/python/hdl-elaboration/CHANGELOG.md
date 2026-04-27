# Changelog

All notable changes to `hdl-elaboration` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — Unreleased

The initial implementation. Bridges the existing Verilog parser to HIR, completing the path from Verilog source → HIR for downstream tooling.

### Added
- `verilog_to_hir`: walks a Verilog AST (from `verilog-parser`) and emits HIR Module, Port, Net, ContAssign, and expression nodes.
- `elaborate_verilog(source, top)`: convenience entrypoint. Parses + elaborates in one call.
- Three-pass design per `hdl-elaboration.md`:
  - Pass 1 — Collect modules into a symbol table.
  - Pass 2 — Bind references, resolve types, type-tag expressions.
  - Pass 3 — Unroll `generate-for` and parameter-driven specializations (basic support).
- Provenance attached to every HIR node (`SourceLang.VERILOG` plus file/line/col).
- Tests covering the canonical 4-bit adder, parameterized modules, hierarchical instances.

### Notes
- Implementation matches the design in `code/specs/hdl-elaboration.md` v0.1.0.
- VHDL frontend, behavioral processes, and full generate-loop unrolling land in 0.2.0.
- Targets the Verilog 2005 grammar (default in `verilog-parser`).
