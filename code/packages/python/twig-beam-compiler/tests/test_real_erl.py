"""End-to-end Twig source → real ``erl`` execution tests.

These tests require ``erl`` on PATH and skip cleanly if it's not.
They are the headline proof that BEAM01 Phases 2-4 work together:
Twig source goes in, byte output comes out of real Erlang/OTP.
"""

from __future__ import annotations

import pytest
from twig_beam_compiler import erl_available, run_source

requires_erl = pytest.mark.skipif(
    not erl_available(),
    reason="erl not on PATH",
)


@requires_erl
def test_addition() -> None:
    """``(+ 1 2)`` → ``main/0`` returns 3 from real erl."""
    result = run_source("(+ 1 2)", module_name="bm_add")
    assert result.returncode == 0, (
        f"erl rejected the module:\n  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )
    # ``erl -eval 'io:format("~p~n", [Result])'`` prints the integer
    # followed by a newline.
    assert result.stdout.strip() == b"3"


@requires_erl
def test_multiplication() -> None:
    """``(* 6 7)`` → 42."""
    result = run_source("(* 6 7)", module_name="bm_mul")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"42"


@requires_erl
def test_subtraction() -> None:
    """``(- 10 3)`` → 7."""
    result = run_source("(- 10 3)", module_name="bm_sub")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"7"


@requires_erl
def test_division() -> None:
    """``(/ 10 2)`` → 5.  BEAM lowering uses ``erlang:div/2`` which
    is integer division, so 10/2 = 5."""
    result = run_source("(/ 10 2)", module_name="bm_div")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"5"


@requires_erl
def test_let_binding() -> None:
    """``(let ((x 5)) (* x x))`` → 25."""
    result = run_source("(let ((x 5)) (* x x))", module_name="bm_let")
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"25"


@requires_erl
def test_nested_arithmetic() -> None:
    """``(+ (* 6 7) (* 2 3))`` → 48."""
    result = run_source(
        "(+ (* 6 7) (* 2 3))", module_name="bm_nested"
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == b"48"
