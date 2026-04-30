# Changelog — beam-bytecode-encoder

## 0.2.0 — 2026-04-29 — BEAM02 Phase 2b (FunT scaffolding)

### Added — ``FunT`` chunk encoding

- ``BEAMFun`` dataclass — one row of the ``FunT`` (function)
  table.  Fields: ``function_atom_index``, ``arity``,
  ``code_label``, ``index``, ``num_free``, ``old_uniq``.
- ``BEAMModule.funs`` field — populates the ``FunT`` chunk;
  the chunk is omitted when empty.
- Validation rules for ``FunT`` rows: rejects out-of-range atom
  indices, ``code_label`` exceeding ``label_count``, non-sequential
  ``index`` values, negative ``num_free`` / ``arity``.
- ``old_uniq`` values wider than 32 bits are silently truncated
  to fit the ``u32`` field — useful when callers compute uniqs
  from a wider hash without downcasting first.

This is infrastructure for the ``make_fun2`` / ``make_fun3``
closure-construction opcodes.  The ir-to-beam Phase 2 lowering
ended up using a list-based closure representation +
``erlang:apply/3`` instead (modern OTP rejects ``make_fun2`` and
``make_fun3`` needs z-tagged extended-list operands), so the
``FunT`` chunk is currently unused by Twig — but the encoder
support stays for future work that wants real BEAM funs.

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
