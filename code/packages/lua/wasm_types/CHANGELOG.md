# Changelog — coding-adventures-wasm-types (Lua)

## [0.1.0] — 2026-03-29

### Added

- `ValType` table with all 7 WebAssembly value type constants
  (i32=0x7F, i64=0x7E, f32=0x7D, f64=0x7C, v128=0x7B, funcref=0x70, externref=0x6F).
- `RefType` table (funcref=0x70, externref=0x6F).
- `BlockType` table (empty=0x40).
- `ExternType` table (func=0, table=1, mem=2, global=3).
- `is_val_type(byte)` — predicate for valid ValType bytes.
- `is_ref_type(byte)` — predicate for reference types.
- `val_type_name(byte)` — human-readable name with "unknown_0xXX" fallback.
- `encode_val_type(vt)` — encode as 1-byte array; errors on invalid input.
- `decode_val_type(bytes, offset)` — decode from byte array; returns
  `{type, bytes_consumed}`.
- `encode_limits(limits)` — encode Limits struct using LEB128 integers;
  flag byte 0x00 for no-max, 0x01 for bounded.
- `decode_limits(bytes, offset)` — decode Limits; returns
  `{limits={min, max}, bytes_consumed}`.
- `encode_func_type(func_type)` — encode function signature with 0x60 magic
  prefix, LEB128 counts, and per-type bytes.
- `decode_func_type(bytes, offset)` — decode function signature; validates
  magic byte and each ValType; returns `{func_type={params, results}, bytes_consumed}`.
- Comprehensive test suite (`tests/test_wasm_types.lua`) with 50+ test cases
  covering all functions, constants, round-trips, and error conditions.
