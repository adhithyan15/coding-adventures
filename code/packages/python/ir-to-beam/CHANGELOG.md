# Changelog — ir-to-beam

## 0.1.0 — 2026-04-29

### Added — BEAM01 Phase 3: compiler-ir → BEAMModule lowering

- ``BEAMBackendConfig`` — configures module name + which IR
  callable region is the entry function.
- ``BEAMBackendError`` — raised on unsupported IR ops or
  structurally invalid programs.
- ``lower_ir_to_beam(ir, config) -> BEAMModule`` — the entry
  point.
- IR-op → BEAM-op coverage (v1):
  - ``LABEL``, ``LOAD_IMM``, ``ADD``, ``SUB``, ``MUL``, ``DIV``,
    ``CALL``, ``RET``.
- Auto-injected ``module_info/0`` and ``module_info/1`` exports
  delegating to ``erlang:get_module_info/{1,2}``.
- Tests:
  - Round-trip parity via ``beam-bytes-decoder`` — every emitted
    module decodes cleanly with matching atom / export / import
    tables.
  - Real-``erl`` smoke tests (skipped without ``erl`` on PATH):
    a synthesised ``add(17, 25)`` returns 42, a synthesised
    ``identity(99)`` returns 99.

### Out of scope (future iterations)

- ``BRANCH_Z`` / ``BRANCH_NZ`` / ``JUMP`` — control flow needs
  live-register tracking.
- ``SYSCALL`` — output / I/O.
- Memory ops (``LOAD_BYTE`` / ``STORE_BYTE``) — Twig doesn't use
  them.
