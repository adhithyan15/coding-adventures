# Changelog

## 0.2.0 — 2026-04-27

**Add Phase G control-flow and Phase 13 hyperbolic-function end-to-end REPL tests.**

Two new test classes added to `tests/test_session.py` (14 tests total):

### `TestPhaseGControlFlow` (8 tests)

Exercises grammar-level control-flow keywords compiled by the Phase G grammar
(`while`, `for..thru`, `for..in`, `block`, `return`, `if/elseif/else`) through
the full REPL stack end-to-end:

- `test_while_loop_sum` — `while s < 5 do s: s + 1` accumulates to 5.
- `test_for_range_sum` — `for i thru 5 do s:s+i` inside a `block` sums to 15.
- `test_for_each_applies_body` — `for x in [1,2,3] do s:s+x` sums to 6.
- `test_if_then_else_true` — `if 2 > 1 then 99 else 0` → 99.
- `test_if_then_else_false` — `if 1 > 2 then 99 else 0` → 0.
- `test_if_then_no_else_miss` — `if false then 99` → false (unmatched branch).
- `test_block_local_scope` — local `x:99` inside block does not overwrite outer `x:10`.
- `test_return_from_block` — `return(42)` inside a block short-circuits to 42.

### `TestPhase13Hyperbolic` (6 tests)

Exercises the Phase 13 hyperbolic function suite (`sinh`, `cosh`, `tanh`,
`asinh`, `acosh`, `atanh`) that was added to `symbolic-vm` 0.32.0:

- `test_sinh_zero` — `sinh(0)` → 0 (exact zero).
- `test_cosh_zero` — `cosh(0)` → 1 (exact one).
- `test_tanh_zero` — `tanh(0)` → 0 (exact zero).
- `test_sinh_numeric` — `ev(sinh(1), numer)` → decimal starting with `1.1` (≈ 1.1752).
- `test_diff_sinh` — `diff(sinh(x), x)` output contains `cosh`.
- `test_integrate_sinh` — `integrate(sinh(x), x)` output contains `cosh`.

## 0.1.0 — 2026-04-25

Initial release — Phase A.

- Interactive REPL invocable via ``python main.py`` or ``python -m main``.
- ``MacsymaLanguage`` plugin that ties together
  ``macsyma-{lexer,parser,compiler}``, the ``symbolic-vm`` evaluator,
  the ``macsyma-runtime`` backend, and the ``cas-pretty-printer``
  output formatter.
- ``MacsymaPrompt`` plugin showing ``(%iN) `` for the global prompt.
- ``Display(expr)`` / ``Suppress(expr)`` wrappers from the compiler are
  inspected before evaluation to decide whether to print results;
  identity-on-inner under the runtime backend.
- ``%``, ``%iN``, ``%oN`` resolve via the ``History`` table installed
  on the backend.
- ``quit;`` and ``:quit`` exit the session; parse / compile / runtime
  errors are caught and the loop continues.
