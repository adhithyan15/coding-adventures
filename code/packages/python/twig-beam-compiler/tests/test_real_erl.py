"""End-to-end Twig source → real ``erl`` execution tests.

Headline TW03 Phase 1 BEAM proof: ``define``, ``if``,
comparison, recursion all work on real Erlang/OTP — completing
parity with JVM01's factorial test on real ``java`` and the
twig-clr-compiler factorial test on real ``dotnet``.
"""

from __future__ import annotations

import pytest

from twig_beam_compiler import erl_available, run_source

requires_erl = pytest.mark.skipif(
    not erl_available(),
    reason="erl not on PATH",
)


# ── Arithmetic ─────────────────────────────────────────────────────────────


@requires_erl
def test_addition() -> None:
    """``(+ 1 2)`` → 3."""
    result = run_source("(+ 1 2)", module_name="bm_add")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"3"


@requires_erl
def test_multiplication() -> None:
    result = run_source("(* 6 7)", module_name="bm_mul")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"42"


@requires_erl
def test_subtraction() -> None:
    result = run_source("(- 10 3)", module_name="bm_sub")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"7"


@requires_erl
def test_division() -> None:
    """``(/ 10 2)`` → 5.  BEAM lowers ``/`` to ``erlang:div/2`` (integer division)."""
    result = run_source("(/ 10 2)", module_name="bm_div")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"5"


@requires_erl
def test_let_binding() -> None:
    result = run_source("(let ((x 5)) (* x x))", module_name="bm_let")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"25"


@requires_erl
def test_nested_arithmetic() -> None:
    result = run_source("(+ (* 6 7) (* 2 3))", module_name="bm_nested")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"48"


# ── if / comparison ────────────────────────────────────────────────────────


@requires_erl
def test_if_taken_branch() -> None:
    result = run_source("(if (= 1 1) 100 200)", module_name="bm_if_t")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"100"


@requires_erl
def test_if_not_taken_branch() -> None:
    result = run_source("(if (= 1 2) 100 200)", module_name="bm_if_f")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"200"


@requires_erl
def test_comparison_lt() -> None:
    result = run_source("(if (< 3 5) 1 0)", module_name="bm_lt")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"1"


@requires_erl
def test_comparison_gt() -> None:
    result = run_source("(if (> 3 5) 1 0)", module_name="bm_gt")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"0"


# ── define + function calls ────────────────────────────────────────────────


@requires_erl
def test_top_level_function() -> None:
    """``(define (square x) (* x x)) (square 7) → 49``."""
    result = run_source(
        "(define (square x) (* x x)) (square 7)",
        module_name="bm_square",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"49"


@requires_erl
def test_two_param_function() -> None:
    result = run_source(
        "(define (add3 a b) (+ a (+ b 3))) (add3 10 20)",
        module_name="bm_add3",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"33"


@requires_erl
def test_top_level_value_define_inlined() -> None:
    result = run_source("(define x 42) x", module_name="bm_val")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"42"


@requires_erl
def test_nested_function_calls() -> None:
    result = run_source(
        """
        (define (inc x) (+ x 1))
        (define (dbl x) (* x 2))
        (inc (dbl 5))
        """,
        module_name="bm_nested_fn",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"11"


# ── recursion ──────────────────────────────────────────────────────────────


@requires_erl
def test_recursion_factorial() -> None:
    """``(fact 5) → 120`` — the headline TW03 Phase 1 BEAM test.

    Proves recursion works on real ``erl``, completing parity
    with JVM01's ``test_recursion_factorial`` on real ``java``
    and twig-clr-compiler's same-named test on real ``dotnet``.
    """
    result = run_source(
        """
        (define (fact n)
          (if (= n 0) 1 (* n (fact (- n 1)))))
        (fact 5)
        """,
        module_name="bm_fact",
    )
    assert result.returncode == 0, (
        f"erl rejected the module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == b"120"


@requires_erl
def test_mutual_recursion_even_odd() -> None:
    """``(even? 4) → 1``.

    Mutual recursion needs both functions to call each other,
    so this exercises CALL across two regions that each push
    args via the Twig calling convention.

    Note: BEAM atoms can't contain ``?`` so we use ``even_q`` /
    ``odd_q`` here (the Twig source uses standard names; the
    *Twig* compiler doesn't sanitise them so we work around
    by avoiding ``?`` in this test).
    """
    result = run_source(
        """
        (define (evenp n) (if (= n 0) 1 (oddp (- n 1))))
        (define (oddp n) (if (= n 0) 0 (evenp (- n 1))))
        (evenp 4)
        """,
        module_name="bm_evenodd",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"1"
