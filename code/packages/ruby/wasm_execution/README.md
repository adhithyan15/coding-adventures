# wasm-execution (Ruby)

WebAssembly 1.0 execution engine for Ruby. Interprets validated WASM modules using the GenericVM infrastructure, implementing all ~182 WASM 1.0 instructions.

## Architecture

```
WasmExecutionEngine
  |
  +-- GenericVM (from virtual-machine)
  |     +-- typed_stack (push_typed/pop_typed)
  |     +-- context execution (execute_with_context)
  |
  +-- Instruction Handlers (registered on GenericVM)
  |     +-- numeric_i32 (33 handlers)
  |     +-- numeric_i64 (32 handlers)
  |     +-- numeric_f32 (23 handlers)
  |     +-- numeric_f64 (23 handlers)
  |     +-- conversion (27 handlers)
  |     +-- variable (5 handlers)
  |     +-- parametric (2 handlers)
  |     +-- memory (27 handlers)
  |     +-- control (13 handlers)
  |
  +-- Decoder (variable-length bytecodes -> fixed-format instructions)
  +-- LinearMemory (byte-addressable, page-based)
  +-- Table (function reference arrays)
  +-- ConstExpr (constant expression evaluator)
```

## Usage

```ruby
require "coding_adventures_wasm_execution"

engine = CodingAdventures::WasmExecution::WasmExecutionEngine.new(
  memory: nil,
  tables: [],
  globals: [],
  global_types: [],
  func_types: [func_type],
  func_bodies: [body],
  host_functions: [nil]
)

result = engine.call_function(0, [WasmExecution.i32(5)])
```

## Dependencies

- wasm-leb128 (LEB128 encoding/decoding)
- wasm-types (WASM type definitions)
- wasm-opcodes (opcode metadata table)
- wasm-module-parser (binary format parser)
- virtual-machine (GenericVM execution engine)

## Development

```bash
bundle install
bundle exec rake test
```
