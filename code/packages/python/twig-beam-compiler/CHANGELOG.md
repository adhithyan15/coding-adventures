# Changelog — twig-beam-compiler

## 0.1.0 — 2026-04-29

### Added — BEAM01 Phase 4: Twig source → real `erl` execution

- ``compile_to_ir(source)`` — Twig → ``IrProgram`` for the v1
  surface (arithmetic expressions, ``let`` bindings, integer
  literals).  The whole program becomes the body of a synthesised
  ``main/0`` function.
- ``compile_source(source)`` — full pipeline: parse → IR emit →
  ir-optimizer → lower_ir_to_beam → encode_beam → ``.beam`` bytes.
- ``run_source(source)`` — drops the ``.beam`` to a temp dir and
  invokes real ``erl -eval`` to call ``main/0`` and capture its
  return value.  Skips when ``erl`` is not on PATH.
- Three real-``erl`` smoke tests (parity with JVM01 in spirit):
  - ``(+ 1 2)`` → main/0 = 3
  - ``(* 6 7)`` → main/0 = 42
  - ``(let ((x 5)) (* x x))`` → main/0 = 25

### Out of scope (future iterations)

- ``define`` of top-level functions; recursion (needs Phase 4.5).
- ``if`` outside of trivial cases (needs branch lowering in
  ``ir-to-beam``).
- I/O / ``SYSCALL`` (the entry function's return value is the
  result for v1).
