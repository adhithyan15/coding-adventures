"""End-to-end Twig source → real ``dotnet`` execution tests.

Headline TW03 Phase 1 proof: ``define``, ``if``, comparison,
recursion all work on real .NET 9.0 — completing CLR parity
with JVM01's factorial test on real ``java``.
"""

from __future__ import annotations

import pytest

from twig_clr_compiler import dotnet_available, run_source

requires_dotnet = pytest.mark.skipif(
    not dotnet_available(),
    reason="dotnet not on PATH",
)


# ── Arithmetic ─────────────────────────────────────────────────────────────


@requires_dotnet
def test_addition() -> None:
    result = run_source("(+ 1 2)", assembly_name="ClrAdd")
    assert result.returncode == 3, result.stderr


@requires_dotnet
def test_multiplication() -> None:
    result = run_source("(* 6 7)", assembly_name="ClrMul")
    assert result.returncode == 42, result.stderr


@requires_dotnet
def test_subtraction() -> None:
    result = run_source("(- 10 3)", assembly_name="ClrSub")
    assert result.returncode == 7, result.stderr


@requires_dotnet
def test_division() -> None:
    result = run_source("(/ 10 2)", assembly_name="ClrDiv")
    assert result.returncode == 5, result.stderr


@requires_dotnet
def test_let_binding() -> None:
    result = run_source("(let ((x 5)) (* x x))", assembly_name="ClrLet")
    assert result.returncode == 25, result.stderr


@requires_dotnet
def test_nested_arithmetic() -> None:
    result = run_source(
        "(+ (* 6 7) (* 2 3))", assembly_name="ClrNested"
    )
    assert result.returncode == 48, result.stderr


# ── if / comparison ────────────────────────────────────────────────────────


@requires_dotnet
def test_if_taken_branch() -> None:
    result = run_source("(if (= 1 1) 100 200)", assembly_name="ClrIfT")
    assert result.returncode == 100, result.stderr


@requires_dotnet
def test_if_not_taken_branch() -> None:
    result = run_source("(if (= 1 2) 100 200)", assembly_name="ClrIfF")
    assert result.returncode == 200, result.stderr


@requires_dotnet
def test_comparison_lt() -> None:
    result = run_source("(if (< 3 5) 1 0)", assembly_name="ClrLt")
    assert result.returncode == 1, result.stderr


@requires_dotnet
def test_comparison_gt() -> None:
    result = run_source("(if (> 3 5) 1 0)", assembly_name="ClrGt")
    assert result.returncode == 0, result.stderr


# ── define + function calls ────────────────────────────────────────────────


@requires_dotnet
def test_top_level_function() -> None:
    """``(define (square x) (* x x)) (square 7) → 49``."""
    result = run_source(
        "(define (square x) (* x x)) (square 7)",
        assembly_name="ClrSquare",
    )
    assert result.returncode == 49, result.stderr


@requires_dotnet
def test_two_param_function() -> None:
    result = run_source(
        "(define (add3 a b) (+ a (+ b 3))) (add3 10 20)",
        assembly_name="ClrAdd3",
    )
    assert result.returncode == 33, result.stderr


@requires_dotnet
def test_top_level_value_define_inlined() -> None:
    result = run_source("(define x 42) x", assembly_name="ClrVal")
    assert result.returncode == 42, result.stderr


@requires_dotnet
def test_nested_function_calls() -> None:
    result = run_source(
        """
        (define (inc x) (+ x 1))
        (define (dbl x) (* x 2))
        (inc (dbl 5))
        """,
        assembly_name="ClrNestedFn",
    )
    assert result.returncode == 11, result.stderr


# ── recursion ──────────────────────────────────────────────────────────────


@requires_dotnet
def test_recursion_factorial() -> None:
    """``(fact 5) → 120`` — the headline TW03 Phase 1 test.

    Proves recursion works on real ``dotnet``, completing parity
    with JVM01's ``test_recursion_factorial`` on real ``java``.
    """
    result = run_source(
        """
        (define (fact n)
          (if (= n 0) 1 (* n (fact (- n 1)))))
        (fact 5)
        """,
        assembly_name="ClrFact",
    )
    assert result.returncode == 120, (
        f"factorial mismatch — exit {result.returncode}, "
        f"stderr={result.stderr!r}"
    )


@requires_dotnet
def test_mutual_recursion_even_odd() -> None:
    """``(even? 4) → 1``."""
    result = run_source(
        """
        (define (even? n) (if (= n 0) 1 (odd? (- n 1))))
        (define (odd? n) (if (= n 0) 0 (even? (- n 1))))
        (even? 4)
        """,
        assembly_name="ClrEvenOdd",
    )
    assert result.returncode == 1, result.stderr


# ── closures (CLR02 Phase 2d) ────────────────────────────────────────────


@requires_dotnet
def test_closure_make_adder() -> None:
    """``((make-adder 7) 35) → 42`` — the headline CLR02 Phase 2d test.

    Exercises the full closure pipeline from real Twig source on
    real ``dotnet``:

    * Free-variable analysis lifts the ``(lambda (x) (+ x n))``
      to a top-level ``_lambda_0`` region with captures-first
      param layout.
    * ``MAKE_CLOSURE`` allocates a ``Closure__lambda_0`` instance
      and stores it via the parallel ``object`` local pool
      (Phase 2c.5 typed register pool).
    * ``APPLY_CLOSURE`` dispatches via
      ``callvirt int32 IClosure::Apply(int32)``.
    * The Closure subclass's ``Apply`` reads the captured ``n``
      from its instance field, adds the explicit arg ``X``, and
      returns the int.
    """
    result = run_source(
        """
        (define (make-adder n) (lambda (x) (+ x n)))
        ((make-adder 7) 35)
        """,
        assembly_name="ClrClosure",
    )
    assert result.returncode == 42, (
        f"closure pipeline broke at runtime.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


@requires_dotnet
def test_closure_let_bound() -> None:
    """``(let ((adder (make-adder 7))) (adder 35)) → 42``.

    Exercises the let-bound closure path: ``adder`` is a local
    binding (not in ``_fn_params``) holding a closure value;
    the call ``(adder 35)`` falls through to APPLY_CLOSURE.
    """
    result = run_source(
        """
        (define (make-adder n) (lambda (x) (+ x n)))
        (let ((adder (make-adder 7))) (adder 35))
        """,
        assembly_name="ClrLetClos",
    )
    assert result.returncode == 42, result.stderr
