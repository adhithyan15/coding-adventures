# Changelog

All notable changes to the `hdl-ir` package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — Unreleased

The initial implementation, built directly from the design in `code/specs/hdl-ir.md`. This is the first implementation layer below the Verilog/VHDL parsers in the silicon stack.

### Added
- HIR type system (`hdl_ir.types`):
  - Scalar types: `TyLogic` (4-state 0/1/X/Z), `TyBit` (2-state), `TyStdLogic` (9-state IEEE 1164), `TyReal`, `TyTime`, `TyString`.
  - Composite types: `TyVector`, `TyInteger`, `TyEnum`, `TyRecord`, `TyArray`, `TyFile`.
  - `width()` helper that projects synthesizable types to bit-widths.
- HIR expression nodes (`hdl_ir.expr`):
  - Atoms: `Lit`, `NetRef`, `VarRef`, `PortRef`.
  - Composites: `Slice`, `Concat`, `Replication`.
  - Operators: `UnaryOp`, `BinaryOp`, `Ternary`.
  - Calls: `FunCall`, `SystemCall`.
  - Attributes: `Attribute` (e.g., `signal'event`).
- HIR statement nodes (`hdl_ir.stmt`):
  - Assignments: `BlockingAssign`, `NonblockingAssign`.
  - Control flow: `IfStmt`, `CaseStmt` (with `CaseItem`), `ForStmt`, `WhileStmt`, `RepeatStmt`, `ForeverStmt`.
  - Suspensions: `WaitStmt`, `DelayStmt`, `EventStmt` (with `Event`).
  - Verification: `AssertStmt`, `ReportStmt`.
  - Misc: `DisableStmt`, `ReturnStmt`, `NullStmt`, `ExprStmt`.
- Module-level constructs (`hdl_ir.module`):
  - `Net` (with `NetKind` enum: `signal`/`wire`/`reg`/`tri`/`wand`/`wor`/`supply0`/`supply1`/`resolved_signal`).
  - `Variable` (process-local with immediate-update semantics).
  - `Port` (with `Direction` enum: `in`/`out`/`inout`/`buffer`).
  - `Process` (with `ProcessKind` enum and `SensitivityItem`).
  - `ContAssign` (continuous / concurrent assignment).
  - `Instance` (module instantiation with parameter binding and port connections).
  - `Module` (top-level circuit definition with `Level` enum: `behavioral`/`structural`).
  - `Parameter`, `Library`.
- Top-level container (`hdl_ir.hir`):
  - `HIR` dataclass with `top` module name + `modules` dict + optional `libraries` dict.
  - `to_json()` / `from_json()` for streaming serialization.
  - `to_dict()` / `from_dict()` for in-memory dict round-trip.
  - JSON schema frozen at v0.1.0; additive evolution.
- Source-language provenance (`hdl_ir.provenance`):
  - `SourceLang` enum: `verilog`/`vhdl`/`ruby_dsl`/`unknown`.
  - `SourceLocation` (file/line/column).
  - `Provenance` carrying lang + location.
- Validation (`hdl_ir.validate`):
  - Rules H1 (top exists), H2 (module names unique), H3 (instance.module resolves), H4 (connection key is a port), H6 (refs resolve), H11 (parameters bound), H12 (structural has no processes), H20 (no self-instantiation).
  - Combinational-loop detection (H10) and width-mismatch (H5) follow in 0.2.0.
  - `ValidationReport` with `errors`, `warnings`, `ok` properties.
- Tests:
  - Round-trip identity for all node types.
  - 4-bit adder construction + validation + JSON.
  - Validation rule positive + negative cases.
  - Mixed-language designs (Verilog top with VHDL leaves).

### Notes
- This package implements the design specified in `code/specs/hdl-ir.md` v0.1.0.
- Downstream packages (`hdl-elaboration`, `hardware-vm`, `synthesis`) consume this IR.
- The IEEE 1076-2008 / 1364-2005 conformance matrix in the spec lists every supported construct and the HIR node it maps to.
- `mypy --strict` passes on the entire `src/` tree.
- Test coverage target: ≥ 85% (libraries: 95%+ aspiration; we converge as more nodes are exercised by downstream tests).
