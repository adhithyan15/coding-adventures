# Changelog — ldp-format

## [0.1.0] — 2026-04-30

### Added — LANG22 PR 11d

- **Pure data crate** for `.ldp` (language-runtime profile) artefacts
  per LANG22 §"Profile artefact format".  Zero external dependencies;
  std-only.  Unblocks three downstream consumers (PR 11e, 11f, 11g)
  to ship in parallel against a stable shared format.
- **`LdpFile`** + supporting types (`Header`, `ModuleRecord`,
  `FunctionRecord`, `InstructionRecord`, `TypeStatus`,
  `PromotionState`, `ObservedKind`).
- **`read<R: Read>(reader) -> Result<LdpFile, LdpReadError>`** —
  parse from any std `Read` source.  Returns typed errors for every
  malformed-input case; never panics.
- **`write<W: Write>(file, writer) -> Result<(), LdpWriteError>`** —
  serialise to any std `Write` sink.  Output is **deterministic** —
  byte-identical input produces byte-identical output (verified by
  `writer_is_deterministic`).
- **String-table deduplication** — identical strings in modules /
  functions / opcodes / type names share one entry.  100-module
  test confirms <150 bytes/module amortised.
- **Forward-compatibility hooks**: `_pad` and `reserved` fields so
  v1.1 / v1.2 writers can add small optional fields without
  breaking v1.0 readers.  All public enums `#[non_exhaustive]`.
- **13 unit tests** covering: round-trips (empty, rich), determinism,
  dedup, all enum variants (`ObservedKind`, `TypeStatus`,
  `PromotionState`), Unicode in names, every error variant.

### Out of scope (future PRs)

- Producers (the JIT / vm-core profiler / explicit profile-collection
  CLI) are separate crates.  They depend on this crate to write.
- Consumers (`aot-with-pgo`, `lang-perf-suggestions`) are separate
  crates.  They depend on this crate to read.
- IC-entry serialisation is reserved-but-unused in v1
  (`ic_entry_count = 0` always).  v2 will populate it once the IC
  vocabulary stabilises.
- SMT-LIB-style textual format (analogue: `.smt2` for SMT) is a
  separate concern; binary is the canonical on-disk shape.
