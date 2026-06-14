# Changelog — coding-adventures-intel4004-backend

## [Unreleased]

### Changed — LANG19: Import CIRInstr from codegen-core

`intel4004_backend.backend` previously imported `CIRInstr` from
`jit_core.cir`.  It now imports from `codegen_core` directly, matching
where `CIRInstr` was moved in LANG19.

- `backend.py`: `from jit_core.cir import CIRInstr` →
  `from codegen_core import CIRInstr`.

- `pyproject.toml`: `coding-adventures-jit-core` replaced by
  `coding-adventures-codegen-core`.

Zero functional change — `CIRInstr` is identical; only the import path
changed.

---

## [0.1.0]

### Added — initial extraction from tetrad-runtime

This package is the new home of the Intel 4004 codegen.  It was
extracted from `tetrad_runtime._intel4004_codegen` (an internal
subpackage of `tetrad-runtime`) so:

1. The codegen has a stable, public, language-agnostic home that any
   LANG-pipeline frontend can import — not just Tetrad.
2. There is a clear pattern (`<arch>-backend` package per target) for
   future siblings: `intel8008-backend`, `intel8080-backend`,
   `mos6502-backend`, `riscv32-backend`, `x86_64-backend`,
   `arm64-backend`, `wasm32-backend`, …

### Public API

- `Intel4004Backend` — `jit_core.BackendProtocol` implementation;
  `compile(cir) -> bytes | None` and `run(binary, args) -> Any`.
- `IRInstr` — internal SSA-by-name IR shape the codegen consumes.
  Re-exported for tests / advanced callers that want to drive the
  codegen directly.
- `codegen(ir)` — IR → 4004 binary (returns `bytes | None`).
- `run_on_4004(binary, args)` — load into `intel4004-simulator`,
  return the u8 accumulator.
- `evaluate_op(op, a, b)` — abstract-evaluation helper used by the
  constant-folding optimiser.
- `DeoptimizerError` — raised by the codegen for unrecoverable
  encoding failures.

### Test coverage

33 tests migrated from `tetrad-runtime/tests/test_intel4004_codegen.py`
plus 5 new `Intel4004Backend` adapter tests covering:

- Protocol conformance (`isinstance(backend, BackendProtocol)`)
- CIR → IRInstr re-projection (type-suffix stripping)
- Deopt cases (type guards, `call_runtime`, codegen exceptions)
- Smoke check that `run` is wired to the simulator

### Why the CIR re-projection still exists

`Intel4004Backend.compile` re-projects `CIRInstr` to an in-package
`IRInstr` shape because the codegen was originally written against
`IRInstr` (in the now-retired `tetrad-jit` package).  A future PR
will rewrite the codegen to consume `CIRInstr` directly and remove
the re-projection — at that point `compile` becomes a one-line
forwarder.
