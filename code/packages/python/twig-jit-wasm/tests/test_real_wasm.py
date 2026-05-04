"""End-to-end tests with the real WASM backend.

These tests prove the full Twig → JIT → WASM → execution pipeline
works.  They use the in-house ``wasm-runtime`` (no external
dependency on Node/wasmtime/etc.) so they always run if the
``wasm-backend`` package is installed.

Equivalence property
====================

For every Twig program in the JIT-compatible subset, the value
returned by ``twig_jit_wasm.run_with_jit(source)`` must equal the
value returned by ``twig.TwigVM().run(source)[1]`` (where ``[1]``
strips the stdout half of TwigVM's return tuple).  This is the
core JIT correctness invariant — JIT is a transparent
*acceleration*, not a behaviour-changing optimization.
"""

from __future__ import annotations

import pytest


@pytest.fixture
def jit_runner():
    """A real ``TwigJITRunner`` with the default WASM backend."""
    from twig_jit_wasm import TwigJITRunner

    return TwigJITRunner()


class TestArithmetic:
    """The core JIT-eligible programs: numeric, no heap, no closure."""

    def test_addition(self, jit_runner) -> None:
        assert jit_runner.run("(+ 1 2)") == 3

    def test_multiplication(self, jit_runner) -> None:
        assert jit_runner.run("(* 6 7)") == 42

    def test_subtraction(self, jit_runner) -> None:
        assert jit_runner.run("(- 10 3)") == 7

    def test_let_with_arithmetic(self, jit_runner) -> None:
        assert jit_runner.run("(let ((x 5)) (* x x))") == 25

    def test_nested_arithmetic(self, jit_runner) -> None:
        assert jit_runner.run("(+ (* 6 7) (* 2 3))") == 48


class TestEquivalenceWithInterpreter:
    """The JIT path must produce the same result as the pure interpreter
    for every program (whether the JIT actually compiled it or fell
    back transparently)."""

    @pytest.mark.parametrize(
        "source",
        [
            "(+ 1 2)",
            "(* 6 7)",
            "(- 100 58)",
            "(let ((x 5)) (* x x))",
            "(+ (* 6 7) (* 2 3))",
            "(define (square x) (* x x)) (square 9)",
            "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)",
        ],
    )
    def test_interpreter_and_jit_agree(self, jit_runner, source: str) -> None:
        from twig import TwigVM

        interpreter_result = TwigVM().run(source)[1]
        jit_result = jit_runner.run(source)
        assert jit_result == interpreter_result, (
            f"divergence on source={source!r}: "
            f"interpreter={interpreter_result!r} jit={jit_result!r}"
        )


class TestClosureFallback:
    """Closure-bearing programs aren't WASM-compileable yet — they
    must still work via interpreter fallback (jit-core's
    deopt-on-fail policy)."""

    def test_closure_returns_correct_value(self, jit_runner) -> None:
        # ``(lambda (x) (+ x 1))`` captures nothing; the inner
        # arithmetic could be JIT'd, but the apply-of-anonymous
        # is heap-mediated so the JIT will skip it.  Either way
        # the answer must be 11.
        result = jit_runner.run("((lambda (x) (+ x 1)) 10)")
        assert result == 11

    def test_capturing_closure_returns_correct_value(self, jit_runner) -> None:
        result = jit_runner.run(
            """
            (define (make-adder n) (lambda (x) (+ x n)))
            ((make-adder 7) 35)
            """
        )
        assert result == 42
