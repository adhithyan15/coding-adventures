# Changelog

## 0.1.0 — PREP01 slice 1: engine core

First release. Includes, conditional compilation, source mapping and resource
bounds, with per-language dialect plug-ins. Macro expansion arrives in slice 2
and is refused with a diagnostic until then rather than silently ignored.

### Added

- `Dialect` trait — the per-language half: directive classification, condition
  evaluation, lexing, and optional stringize/paste hooks that default to
  "unsupported" so the engine cannot assume they exist.
- `SourceFs` trait with `MemoryFs` (tests, fully in-memory) and `RootedFs`
  (production, confined to declared search roots).
- `SourceMap` / `Locus` — per-token provenance in a side table, with the
  expansion chain interned parent-id style so the map is `O(tokens +
  expansions)` rather than `O(tokens x depth)`.
- `Bounds` — finite defaults, tighten-only for dialects *and* embedding hosts,
  covering include depth and cycles, total inclusions, source bytes, per-file
  bytes, tokens produced, token spelling length, synthesised text, grouping and
  conditional nesting, diagnostics, and a `fuel` catch-all.
- One-pass `preprocess` in which inclusion and conditional selection interleave,
  traversed with explicit stacks rather than native recursion.

### Notes on two things that are easy to get wrong

- **Skipped groups are inert.** Inside `@if 0` the engine tracks nesting only:
  no condition evaluation, no include resolution, no expansion. Otherwise every
  bomb the bounds guard against is reachable from the one place a reviewer
  stops reading.
- **No native recursion.** In Rust a stack overflow is an abort, not a
  catchable panic, so a recursive traversal would convert a depth *diagnostic*
  into a process kill and defeat the bounds entirely.

### Fixed during development

- `RootedFs` rejected `/etc/passwd` on Unix but not on Windows, because
  `Path::is_absolute()` is `false` for a root-relative path there (Windows
  requires a drive prefix). The gate now checks `has_root()` as well. Caught by
  the crate's own test suite on the platform that matters most here.
