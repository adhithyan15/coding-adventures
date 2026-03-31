# Changelog — CodingAdventures::WasmSimulator

## 0.01 — 2026-03-29

### Added

- Initial implementation of WebAssembly stack machine simulator
- `new($module)` — instantiate from a WasmModuleParser-parsed module
- `call($name, @args)` — call exported function by name
- `call_by_index($idx, @args)` — call function by 0-based module index
- `get_global($name)` / `set_global($name, $value)` — exported global access
- `memory_read($offset, $len)` / `memory_write($offset, \@bytes)` — linear memory API
- Supported instructions:
  - Constants: `i32.const`, `i64.const`
  - Arithmetic: `i32.add`, `i32.sub`, `i32.mul`, `i32.div_s`, `i32.div_u`, `i32.rem_s`, `i32.rem_u`
  - Bitwise: `i32.and`, `i32.or`, `i32.xor`, `i32.shl`, `i32.shr_s`, `i32.shr_u`
  - Comparison: `i32.eq`, `i32.ne`, `i32.lt_s`, `i32.lt_u`, `i32.le_s`, `i32.le_u`, `i32.gt_s`, `i32.gt_u`, `i32.ge_s`, `i32.ge_u`, `i32.eqz`
  - Memory: `i32.load`, `i32.store`, `memory.size`, `memory.grow`
  - Control: `nop`, `unreachable`, `block`, `loop`, `if`, `else`, `end`, `br`, `br_if`, `return`, `call`
  - Variable: `local.get`, `local.set`, `local.tee`, `global.get`, `global.set`
  - Stack: `drop`, `select`
- `to_i32()` / `to_u32()` helpers for 32-bit wrapping arithmetic
- Portable bitwise helpers using Perl 5.26+ integer operators
- Label stack for structured control flow (block/loop/if/br/br_if)
- Activation frames: parameters + zero-initialized local variables
- Global initialization from constant expressions
- Linear memory allocation from memory section; data segment initialization
- Memory bounds checking with descriptive die messages
- Division-by-zero and INT_MIN/-1 traps for `i32.div_s`
- Comprehensive test suite (t/00-load.t, t/01-basic.t) covering:
  - Constant push, local variable get/set/tee
  - All arithmetic operations with overflow/wrap cases
  - All comparison operations (signed and unsigned)
  - Bitwise operations
  - Memory store/load roundtrip, size, grow
  - Global variable read/write
  - Block, loop, if/else control flow
  - br and br_if branching (taken and not-taken)
  - Loop countdown with br_if inside loop
  - Direct function calls (double → quad)
  - Recursive fibonacci (stress test for call stack)
  - Error cases: OOB memory, divide-by-zero, unreachable, unknown exports
