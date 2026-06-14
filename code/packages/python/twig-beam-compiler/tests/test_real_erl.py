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


# ── closures (TW03 Phase 2) ────────────────────────────────────────────────


@requires_erl
def test_closure_make_adder() -> None:
    """``((make-adder 7) 35) → 42`` — the headline TW03 Phase 2 test.

    Exercises the full closure pipeline end-to-end on real ``erl``:

    * Free-variable analysis lifts the ``(lambda (x) (+ x n))``
      to a top-level ``_lambda_0/2`` that takes ``n`` (captured)
      then ``x`` (explicit).
    * ``MAKE_CLOSURE`` builds a ``[FnAtom | [n]]`` cons cell.
    * The outer call ``((make-adder 7) ...)`` uses
      ``APPLY_CLOSURE`` because the function position is itself
      an ``Apply`` (not a known top-level name).
    """
    result = run_source(
        """
        (define (make-adder n) (lambda (x) (+ x n)))
        ((make-adder 7) 35)
        """,
        module_name="bm_closure",
    )
    assert result.returncode == 0, (
        f"erl rejected the module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == b"42"


@requires_erl
def test_closure_let_bound() -> None:
    """``(let ((adder (make-adder 7))) (adder 35)) → 42``.

    Exercises the let-bound closure path: ``adder`` is a local
    binding (not in ``_fn_params``) holding a closure value; the
    call ``(adder 35)`` drops through to the APPLY_CLOSURE branch.
    """
    result = run_source(
        """
        (define (make-adder n) (lambda (x) (+ x n)))
        (let ((adder (make-adder 7))) (adder 35))
        """,
        module_name="bm_letclos",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"42"


# ── Heap primitives (TW03 Phase 3e for BEAM) ──────────────────────────────


@requires_erl
def test_heap_list_of_ints_length() -> None:
    """``(length (cons 1 (cons 2 (cons 3 nil)))) → 3`` runs end-to-end
    on real ``erl`` from raw Twig source.

    The headline TW03 Phase 3 acceptance criterion — Twig parser
    → IR emitter → BEAM lowering → real ``erl``.  Builds the
    cons-cell list via ``MAKE_CONS``, walks via ``CDR`` + ``IS_NULL``
    (lowered to BEAM ``get_tl`` + ``is_nil``), recurses through
    ``length``.  BEAM cons cells are first-class native terms with
    their own GC so this works without any JVM-style obj-pool
    caller-saves workaround.
    """
    result = run_source(
        """
        (define (length xs)
          (if (null? xs) 0 (+ 1 (length (cdr xs)))))
        (length (cons 1 (cons 2 (cons 3 nil))))
        """,
        module_name="bm_length",
    )
    assert result.returncode == 0, (
        f"length pipeline broke at runtime.\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    assert result.stdout.strip() == b"3"


@requires_erl
def test_heap_car_returns_int() -> None:
    """``(car (cons 42 nil)) → 42`` exercises the int-from-cons-head
    path."""
    result = run_source(
        "(car (cons 42 nil))",
        module_name="bm_carone",
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"42"


@requires_erl
def test_heap_quoted_symbol_returns_atom() -> None:
    """``'foo`` returns the atom ``foo``.  Real ``erl`` prints atoms
    unquoted when the name is a valid identifier."""
    result = run_source("'foo", module_name="bm_symfoo")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"foo"
