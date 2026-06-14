# Changelog — debug-sidecar

All notable changes to this crate will be documented here.

## [0.1.0] — 2026-04-28

### Added

- **`DebugSidecarWriter`** — append-only builder that records IIR instruction-to-source-location
  mappings for a set of named functions.
  - `new()` — creates an empty writer.
  - `add_source_file(path, checksum)` — registers a source file; returns a `usize` file ID.
  - `begin_function(fn_name, start_instr, param_count)` — opens a function entry.
  - `end_function(fn_name, end_instr)` — closes a function entry with its last instruction index.
  - `record(fn_name, instr_index, file_id, line, col)` — appends a `(instr_index → location)` row.
  - `declare_variable(fn_name, reg_index, name, type_hint, live_start, live_end)` — records a
    variable's live range within a function.
  - `finish()` — serialises the entire sidecar to `Vec<u8>` (JSON via `serde_json`).

- **`DebugSidecarReader`** — query engine that deserialises and indexes the sidecar.
  - `new(data: &[u8])` — parses JSON; returns `Err(SidecarError)` on malformed input.
  - `lookup(fn_name, instr_index)` — DWARF-style "last row ≤ N" bisect via `partition_point`;
    returns `Option<SourceLocation>`.
  - `find_instr(file, line)` — reverse lookup; returns `Option<usize>` instruction index.
  - `live_variables(fn_name, at_instr)` — returns all `Variable`s whose live range covers
    `at_instr`.
  - `source_files()` — returns the list of registered source file paths.
  - `function_names()` — returns all known function names.
  - `function_range(fn_name)` — returns `(start_instr, end_instr)` for a function.
  - `raw_line_rows(fn_name)` — returns the pre-sorted `&[LineRow]` slice; used by
    `native-debug-info` to build `.debug_line` sections.

- **`SourceLocation`** — frozen `(file: String, line: u32, col: u32)` triple.
  - `Display` prints `"file:line:col"`.

- **`Variable`** — register binding with `name: String`, `type_hint: String`,
  `reg_index: u32`, `live_start: usize`, `live_end: usize`.
  - `is_live_at(instr_index: usize) -> bool` convenience predicate.

- **`LineRow`** — `pub struct` with fields `instr_index: usize, file_id: usize,
  line: u32, col: u32`; exported so `native-debug-info` can pattern-match it.

- **`SidecarError`** — `pub struct SidecarError(pub String)` implementing `Display` and
  `std::error::Error`.

- 43 tests: 38 unit tests (types × 10, writer × 11, reader × 17) + 5 doc-tests.
