# Changelog — coding-adventures-wasm-opcodes (Lua)

## [0.1.0] — 2026-03-29

### Added

- `OPCODES` table mapping 50+ opcode byte values to `{name, operands}` entries,
  covering all MVP opcode categories:
  - Control flow (unreachable, nop, block, loop, if, else, end, br, br_if,
    br_table, return, call, call_indirect)
  - Parametric (drop, select)
  - Variable (local.get, local.set, local.tee, global.get, global.set)
  - Memory loads (i32.load, i64.load, f32.load, f64.load, i32.load8_s/u,
    i32.load16_s/u, i64.load8_s/u)
  - Memory stores (i32.store, i32.store8, i32.store16)
  - Memory size (memory.size, memory.grow)
  - i32 numeric (const, eqz, eq, ne, lt_s, add, sub, mul, div_s, and, or,
    xor, shl, shr_s)
  - i64 numeric (const, add, sub, mul)
  - f32 numeric (const, add, sub, mul)
  - f64 numeric (const, add, sub, mul)
  - Conversions (i32.wrap_i64, i32.trunc_f32_s, i64.extend_i32_s,
    f32.demote_f64, f64.promote_f32)
- `opcode_name(byte)` — returns mnemonic or "unknown_0xXX" fallback.
- `is_valid_opcode(byte)` — predicate for known opcodes.
- `get_opcode_info(byte)` — returns full `{name, operands}` table or nil.
- Comprehensive test suite (`tests/test_wasm_opcodes.lua`) covering all
  opcode categories, unknown byte handling, and structural invariants.
