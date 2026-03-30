# Changelog — CodingAdventures::WasmTypes (Perl)

## [0.01] — 2026-03-29

### Added

- `%ValType` hash with all 7 WebAssembly value type constants
  (i32=0x7F, i64=0x7E, f32=0x7D, f64=0x7C, v128=0x7B, funcref=0x70, externref=0x6F).
- `%RefType` hash (funcref=0x70, externref=0x6F).
- `%BlockType` hash (empty=0x40).
- `%ExternType` hash (func=0, table=1, mem=2, global=3).
- Perl `use constant` definitions for all ValType bytes.
- `is_val_type($byte)` — predicate using hash lookup.
- `is_ref_type($byte)` — predicate for reference types.
- `val_type_name($byte)` — human-readable name with "unknown_0xXX" fallback.
- `encode_val_type($vt)` — returns a one-element list; croaks on invalid input.
- `decode_val_type($aref, $offset)` — 0-based offset; returns (type, count).
- `encode_limits(\%lim)` — flag byte 0x00/0x01 + LEB128 integers.
- `decode_limits($aref, $offset)` — returns (hashref, bytes_consumed).
- `encode_func_type(\%ft)` — 0x60 magic + LEB128 counts + type bytes.
- `decode_func_type($aref, $offset)` — validates magic, types; returns
  (hashref with params/results arrayrefs, bytes_consumed).
- Test suite (`t/00-load.t`, `t/01-basic.t`) with 70+ assertions covering all
  constants, functions, round-trips, and error conditions.
