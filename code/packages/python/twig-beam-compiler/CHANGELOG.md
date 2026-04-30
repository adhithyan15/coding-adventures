# Changelog — twig-beam-compiler

## 0.3.0 — 2026-04-29 — TW03 Phase 2 (BEAM closures)

### Added — anonymous lambdas and closure invocation

- Anonymous ``(lambda (x) body)`` forms in expression position
  are lifted to fresh ``_lambda_N`` top-level regions.  Free-
  variable analysis (via ``twig.free_vars``) determines what
  each lifted lambda captures; the use site emits
  ``MAKE_CLOSURE`` with the current values of those captures.
- ``Apply`` whose ``fn`` slot is anything other than a known
  builtin or top-level function name lowers to
  ``APPLY_CLOSURE``.  This covers calls of let-bound closures,
  closures returned from other functions, and chained calls
  like ``((make-adder 7) 35)``.
- ``BEAMBackendConfig.closure_free_var_counts`` is populated
  from the lifted-lambda table so ir-to-beam knows the
  captures-first parameter layout for each lifted region.

### Test additions (54 tests total, 89.56% coverage)

- ``test_lambda_lifts_to_top_level_region`` — IR-shape unit
  test covering MAKE_CLOSURE emission for a captured value.
- ``test_closure_call_emits_apply_closure`` — IR-shape unit
  test for chained closure invocation.
- ``test_closure_make_adder`` — end-to-end on real ``erl``:
  ``((make-adder 7) 35) → 42``.
- ``test_closure_let_bound`` — end-to-end:
  ``(let ((adder (make-adder 7))) (adder 35)) → 42``.

## 0.2.0 — 2026-04-29 — TW03 Phase 1 (BEAM)

Closes the language-surface gap on the BEAM backend so Twig
source has the same expressive power on real ``erl`` as it does
on real ``java`` (since JVM01) and real ``dotnet`` (since the
Phase 1 CLR PR).

### Added — full TW03 Phase 1 surface

- Top-level ``define`` for both functions and value constants
- Function calls (incl. multi-arg, nested, value-returning)
- Recursion (incl. mutual recursion)
- ``if`` / ``else``
- Comparison: ``=``, ``<``, ``>``

### Changed — register convention to match JVM/CLR

- Result lives in ``r1`` (was implicit before)
- Function params arrive in ``r2..r{arity+1}``
- Holding registers start at ``r10``

### Test additions (51 tests total, 88% coverage)

- 6 arithmetic + ``let``
- 4 ``if`` / comparison
- 4 ``define`` + multi-arg + nested calls + value-define inlining
- **2 recursion**: ``(fact 5) → 120`` and ``(evenp 4) → 1`` mutual
  recursion
- All confirmed passing on local Erlang/OTP

### Out of scope at the time (closures landed in 0.3.0; the rest still pending)

- Heap primitives (``cons`` / ``car`` / ``cdr`` / symbols / quote)
- Explicit I/O via ``erlang:put_chars/1`` (today the result
  channel is the function return value printed by ``-eval``)

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
