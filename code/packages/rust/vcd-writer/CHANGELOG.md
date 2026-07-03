# Changelog — vcd-writer

## [0.1.0] — 2026-06-13

### Added

- `VcdWriter::new(timescale)` — initialises writer with date/version/timescale preamble
- `open_scope` / `open_scope_kind` / `close_scope` — hierarchical scope support
- `declare(name, width, kind)` — allocates a compact base-94 VCD variable identifier
- `end_definitions()` — emits `$enddefinitions $end`; auto-called before first `time()`
- `time(t)` — emits `#t` timestamp; no-op if already at `t`
- `dump_initial(values)` — emits `$dumpvars` block
- `value_change(id, val)` — emits value change; skips if value unchanged (dedup)
- `value_change_at(t, id, val)` — convenience for advance-time + change
- `finish()` / `text()` — retrieve accumulated VCD text
- `IdAllocator` — base-94 printable-ASCII ID allocation (`!` through `~`, then `!!`, etc.)
- `SignalEvent` struct + `attach()` helper for `hardware-vm` callback integration
- 16 integration tests covering header, scopes, declarations, timestamps, value formatting, dedup, attach
- 1 doc-test in `lib.rs`
