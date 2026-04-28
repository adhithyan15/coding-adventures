# Changelog

## [0.1.0] — Unreleased

Initial implementation of the VCD waveform writer per IEEE 1364-2005 §18.

### Added
- `VcdWriter`: streaming text writer with `open_scope` / `close_scope` / `declare` / `end_definitions` / `time` / `value_change` / `dump_initial`. Context-manager support.
- `_IdAllocator`: printable-ASCII compact identifier generation (94-char base, single char for the first 94 vars, 2-char for next ~9000).
- `Scope`, `VarDef` data classes for hierarchy tracking.
- `attach_to_callback_emitter`: returns a `hardware-vm`-compatible callback that maps `Event(time, signal, new_value)` to `writer.value_change(...)`.
- 4-state value support: scalar `0`/`1`/`x`/`z`; vector binary strings; real values via `r<n>`.
- Time-monotonicity check (raises if time goes backward).
- Idempotence: emitting an unchanged value is a no-op (per VCD spec).

### Conformance (IEEE 1364-2005 §18)
- `$date`, `$version`, `$timescale`: full
- `$scope` / `$upscope` / `$enddefinitions`: full
- `$var wire/reg/integer/real`: full
- `#t` time markers, scalar and vector value changes: full
- `$dumpvars` initial dump: full
- Extended VCD (4-state with strength): out of scope; future spec.
- FST output: out of scope; future spec.
