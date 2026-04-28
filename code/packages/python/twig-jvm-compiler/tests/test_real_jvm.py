"""End-to-end tests for ``twig-jvm-compiler`` running on real ``java``.

These tests:

1. Compile Twig source to a real JVM ``.class`` file via the
   in-house ``ir-to-jvm-class-file`` pipeline.
2. Write the file to a fresh temp dir.
3. Invoke ``java -cp <tmp> <ClassName>`` as a subprocess.
4. Assert on the captured stdout bytes.

The tests skip cleanly when ``java`` isn't on PATH — same pattern
the existing ``test_oct_8bit_e2e.py`` uses for Oct-on-JVM tests.
"""

from __future__ import annotations

import pytest

from twig_jvm_compiler import java_available, run_source

requires_java = pytest.mark.skipif(
    not java_available(),
    reason="'java' binary not found on PATH",
)


@requires_java
def test_arithmetic_addition() -> None:
    """``(+ 1 2)`` writes byte 3 to stdout."""
    result = run_source("(+ 1 2)", class_name="TwigAdd")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([3])


@requires_java
def test_arithmetic_subtraction() -> None:
    result = run_source("(- 10 3)", class_name="TwigSub")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([7])


@requires_java
def test_arithmetic_multiplication() -> None:
    result = run_source("(* 6 7)", class_name="TwigMul")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([42])


@requires_java
def test_let_binding() -> None:
    result = run_source("(let ((x 5)) (* x x))", class_name="TwigLet")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([25])


@requires_java
def test_if_taken() -> None:
    result = run_source("(if (= 1 1) 100 200)", class_name="TwigIfT")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([100])


@requires_java
def test_if_not_taken() -> None:
    result = run_source("(if (= 1 2) 100 200)", class_name="TwigIfF")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([200])


@requires_java
def test_top_level_function_call() -> None:
    """``(define (square x) (* x x)) (square 7)`` writes byte 49."""
    src = "(define (square x) (* x x)) (square 7)"
    result = run_source(src, class_name="TwigSquare")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([49])


@requires_java
def test_function_with_two_params() -> None:
    """Verifies the param-slot calling convention for arity > 1."""
    src = "(define (add3 a b) (+ a (+ b 3))) (add3 10 20)"
    result = run_source(src, class_name="TwigAdd3")
    assert result.returncode == 0, result.stderr
    # 10 + 20 + 3 = 33
    assert result.stdout == bytes([33])


@requires_java
@pytest.mark.xfail(
    reason=(
        "Recursion is broken under the current ir-to-jvm-class-file "
        "calling convention.  That backend stores all 'registers' in "
        "a class-level static int array shared across every method "
        "invocation — so when fact(5) calls fact(4), fact(5)'s own "
        "parameter register r2 gets overwritten with 4, and the "
        "outer multiplication uses 4 instead of 5.  The fix lives in "
        "ir-to-jvm-class-file (use real per-method JVM locals instead "
        "of a static array), which is a separate infrastructure PR.  "
        "Tracked alongside CLR01 as the 'real-runtime correctness' "
        "track.  When that lands, this xfail reverts to a passing test."
    ),
    strict=True,
)
def test_recursion_factorial_small() -> None:
    """Documented xfail: recursion under the static-register-array
    convention is broken.  See marker reason for the gap and fix."""
    src = """
        (define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))
        (fact 5)
    """
    result = run_source(src, class_name="TwigFact")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([120])


@requires_java
def test_top_level_value_define_inlined() -> None:
    """``(define x 42)`` value-defines fold to compile-time constants."""
    src = "(define x 42) x"
    result = run_source(src, class_name="TwigVal")
    assert result.returncode == 0, result.stderr
    assert result.stdout == bytes([42])


@requires_java
def test_nested_function_calls() -> None:
    """``(f (g x))`` — the holding-register convention must
    survive a nested call without clobbering arg slots."""
    src = """
        (define (inc x) (+ x 1))
        (define (dbl x) (* x 2))
        (inc (dbl 5))
    """
    result = run_source(src, class_name="TwigNested")
    assert result.returncode == 0, result.stderr
    # dbl(5) = 10, inc(10) = 11
    assert result.stdout == bytes([11])
