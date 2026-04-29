# Changelog — beam-bytecode-encoder

## 0.1.0 — 2026-04-29

### Added — BEAM01 Phase 2: pure encoder

- ``BEAMModule`` / ``BEAMInstruction`` / ``BEAMOperand`` /
  ``BEAMImport`` / ``BEAMExport`` dataclasses describing a BEAM
  module structurally.
- ``encode_beam(module: BEAMModule) -> bytes`` — produces a
  byte-exact ``.beam`` IFF container.
- Chunk encoders: ``AtU8``, ``Code``, ``StrT``, ``ImpT``,
  ``ExpT``, ``LocT``.
- Compact-term operand encoding (3-bit type tag + length-prefix
  per ECMA-style BEAM file format reference).
- Round-trip tests via ``beam-bytes-decoder``: encoded modules
  decode cleanly and produce the same structural representation.
- Real-``erl`` load test (skipped when ``erl`` not on PATH):
  the smallest valid module loads via ``code:load_file/1``.

### Out of scope (future phases)

- ``LitT`` / ``FunT`` / ``Line`` / ``Attr`` / ``CInf`` chunks —
  not needed for the smallest loadable module.
- IR → BEAM lowering (BEAM01 Phase 3, separate
  ``ir-to-beam`` package).
- Twig source → ``.beam`` (BEAM01 Phase 4, separate
  ``twig-beam-compiler`` package).
