# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Complete WebAssembly 1.0 execution engine with all ~182 instruction handlers.
- WasmValue type system: typed constructors i32(), i64(), f32(), f64() with
  proper wrapping semantics (i32 via `|0`, i64 via BigInt.asIntN, f32 via Math.fround).
- LinearMemory: page-based (64 KiB) byte-addressable memory with DataView-backed
  load/store for all WASM widths (8/16/32/64-bit, signed/unsigned).
- Table: function reference table for call_indirect with grow support.
- HostInterface protocol for import resolution (functions, globals, memory, tables).
- TrapError for unrecoverable WASM runtime errors (div-by-zero, OOB, unreachable).
- Bytecode decoder: pre-instruction hook that converts variable-length WASM
  bytecodes to GenericVM's fixed-format Instruction objects.
- Control flow map builder: one-time O(n) pre-scan mapping block/loop/if → end/else.
- Constant expression evaluator for global initializers and segment offsets.
- 9 instruction handler modules organized by category:
  - numeric_i32: 33 handlers (arithmetic, comparison, bitwise with wrapping)
  - numeric_i64: 32 handlers (BigInt-based, 64-bit wrapping)
  - numeric_f32: 23 handlers (Math.fround precision, NaN handling, banker's rounding)
  - numeric_f64: 23 handlers (native JS number IEEE 754)
  - conversion: 27 handlers (wrap, extend, trunc, convert, reinterpret, promote/demote)
  - variable: 5 handlers (local.get/set/tee, global.get/set)
  - parametric: 2 handlers (drop, select)
  - memory: 27 handlers (all load/store variants + memory.size/grow)
  - control: 13 handlers (block, loop, if/else/end, br, br_if, br_table, return, call, call_indirect)
- WasmExecutionEngine: orchestrates GenericVM with registered handlers and context.
- Built on GenericVM's new typed stack and context-aware handler infrastructure.
- 200 unit tests across 12 test files covering all instruction categories.
