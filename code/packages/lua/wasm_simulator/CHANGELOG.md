# Changelog — coding-adventures-wasm-simulator

## 0.1.0 — 2026-03-29

### Added

- Initial implementation of WebAssembly stack machine simulator
- `Instance.new(module)` — instantiate from a parsed wasm_module_parser module
- `instance:call(name, args)` — call exported function by name
- `instance:call_by_index(idx, args)` — call function by 0-based module index
- `instance:get_global(name)` / `instance:set_global(name, value)` — global variable access
- `instance:memory_read(offset, len)` / `instance:memory_write(offset, bytes)` — linear memory host API
- Supported instructions:
  - Numeric: `i32.const`, `i64.const`
  - Arithmetic: `i32.add`, `i32.sub`, `i32.mul`, `i32.div_s`, `i32.div_u`, `i32.rem_s`, `i32.rem_u`
  - Bitwise: `i32.and`, `i32.or`, `i32.xor`, `i32.shl`, `i32.shr_s`, `i32.shr_u`
  - Comparison: `i32.eq`, `i32.ne`, `i32.lt_s`, `i32.lt_u`, `i32.le_s`, `i32.le_u`, `i32.gt_s`, `i32.gt_u`, `i32.ge_s`, `i32.ge_u`, `i32.eqz`
  - Memory: `i32.load`, `i32.store`, `memory.size`, `memory.grow`
  - Control: `nop`, `unreachable`, `block`, `loop`, `if`, `else`, `end`, `br`, `br_if`, `return`, `call`
  - Variable: `local.get`, `local.set`, `local.tee`, `global.get`, `global.set`
  - Stack: `drop`, `select`
- 32-bit wrapping arithmetic via `to_i32()` helper
- Label stack for structured control flow (block/loop/if/br/br_if)
- Activation frames with parameters and zero-initialized local variables
- Global variable initialization from constant init expressions
- Linear memory allocation from memory section; data segment initialization
- Memory bounds checking with descriptive trap messages
- Division-by-zero and INT_MIN/-1 overflow traps for `i32.div_s`
- Comprehensive test suite covering:
  - Constant push (`i32.const`)
  - Local variable get/set/tee roundtrips
  - All supported arithmetic operations with overflow/wrap cases
  - All supported comparison operations (signed)
  - Bitwise operations with truth-table cases
  - Memory load/store roundtrip
  - Memory size and grow instructions
  - Global variable read/write (mutable and immutable)
  - Block, loop, if/else control flow
  - br and br_if branching
  - Loop with countdown (br_if inside loop)
  - Recursive function calls (fibonacci)
  - Error cases: out-of-bounds access, divide by zero, unreachable
