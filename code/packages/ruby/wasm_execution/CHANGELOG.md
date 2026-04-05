# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Complete WASM 1.0 execution engine with all ~182 instruction handlers
- WasmValue constructors (i32, i64, f32, f64) with proper wrapping semantics
- Type extraction helpers (as_i32, as_i64, as_f32, as_f64) with trap-on-mismatch
- LinearMemory: byte-addressable memory with page-based growth, all load/store variants
- Table: function reference arrays for indirect calls (call_indirect)
- TrapError: custom exception for WASM runtime traps
- HostInterface module and HostFunction struct for import resolution
- Bytecode decoder: variable-length WASM bytecodes to fixed-format instructions
- Control flow map builder: one-time pre-scan for block/loop/if end targets
- Constant expression evaluator for global/data/element initializers
- Instruction handlers: numeric i32 (33), i64 (32), f32 (23), f64 (23), conversion (27), variable (5), parametric (2), memory (27), control flow (13)
- WasmExecutionEngine: interprets validated WASM modules via GenericVM
