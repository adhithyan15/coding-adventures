# Changelog

## [0.1.0] - 2026-04-23

### Added

- `DebugSidecarWriter` — accumulates source-location data during compilation:
  - `add_source_file(path, checksum)` — registers source files with deduplication
  - `begin_function` / `end_function` — records instruction ranges per function
  - `record(fn_name, instr_index, *, file_id, line, col)` — one entry per emitted IIR instruction
  - `declare_variable(fn_name, *, reg_index, name, type_hint, live_start, live_end)` — variable live-range bindings
  - `finish() → bytes` — serializes to JSON UTF-8 bytes (format is internal; swap to binary later without changing callers)

- `DebugSidecarReader` — answers debug queries from a compiled sidecar:
  - `lookup(fn_name, instr_index) → SourceLocation | None` — DWARF-style bisect lookup; returns location of the nearest preceding recorded instruction
  - `find_instr(file, line) → int | None` — reverse lookup for setting breakpoints; returns the lowest matching instruction index across all functions
  - `live_variables(fn_name, at_instr) → list[Variable]` — variable inspection for the Variables panel; sorted by register index
  - `source_files()`, `function_names()`, `function_range()` — metadata accessors

- `SourceLocation` — frozen hashable `(file, line, col)` triple; `str()` returns `"file:line:col"`

- `Variable` — frozen dataclass with `reg_index`, `name`, `type_hint`, `live_start`, `live_end`; `is_live_at(instr_index)` helper

- `__init__.py` re-exports all public types from the top-level package

- Comprehensive test suite covering writer serialization, reader round-trips, `lookup()` with DWARF-style coverage semantics, `find_instr()` cross-function scans, `live_variables()` range filtering, and error handling for corrupt/wrong-version sidecars
