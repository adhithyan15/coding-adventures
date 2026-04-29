# Changelog — ir-to-beam

## 0.2.0 — 2026-04-29 — TW03 Phase 1 (BEAM)

### Added — branching, comparison, and recursion

- ``BRANCH_Z`` / ``BRANCH_NZ`` lowering via BEAM's ``is_ne_exact`` /
  ``is_eq_exact`` (whose "fall through if condition holds, jump
  otherwise" semantics flip to direct BRANCH semantics with
  appropriate operand choice).
- ``JUMP`` lowering to BEAM's ``jump`` opcode.
- ``CMP_EQ`` / ``CMP_LT`` / ``CMP_GT`` lowering — each becomes a
  5-instruction if-then-else pattern (``is_X`` + move 1 + jump +
  label + move 0 + label).  ``CMP_GT`` uses ``is_lt`` with swapped
  operands since BEAM has no ``is_gt``.
- ``ADD_IMM`` lowering — single ``move`` for ``imm == 0`` (the
  Twig MOV idiom), ``gc_bif2`` otherwise.
- Internal LABEL handling: ``_split_callable_regions`` now
  distinguishes callable regions (CALL targets + entry) from
  internal labels (``_else_*``, ``_endif_*``); internal labels
  stay in the body and emit ``label N`` opcodes.

### Changed — y-register frame for recursion support

- Function bodies now use BEAM **y-registers** (callee-saves stack)
  for all IR registers, with **x-registers** used only at CALL
  boundaries.  This is necessary for recursive functions because
  BEAM x-registers are caller-saves (clobbered across calls).
- Each function emits ``allocate K, arity`` at entry (K = max IR
  register index used) and ``deallocate K`` before ``return``.
- At function entry, args are copied from BEAM ``x0..x{arity-1}``
  into the IR's param-slot registers ``y2..y{arity+1}``.
- At each CALL site, args are copied from ``y{2+i}`` to ``x{i}``,
  then ``call N, label``, then the result is copied from ``x0``
  back into the IR's HALT-result register ``y1``.

### Added tests

- ``test_recursive_factorial_returns_120`` — hand-built IR
  exercising ``allocate`` + ``call`` + ``deallocate`` +
  ``BRANCH_Z`` + ``CMP_EQ`` + arithmetic across recursive call
  boundaries.  Confirms ``fact(5) → 120`` on real ``erl``.
- Tests for each new IR-op lowering.

### Migration notes for existing callers

The IR convention is now **r1 holds the function's return value**
(matches twig-jvm-compiler / twig-clr-compiler).  Programs that
used ``r0`` as the result must update.

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
