"""End-to-end integration tests for ``twig-clr-compiler``.

Each test:

1. Compiles Twig source to a PE/CLI assembly.
2. Runs that assembly on the in-house ``clr-vm-simulator``.
3. Asserts on the program's return value.

These tests are the proof-of-life for the full TW02 pipeline:
parse → AST → IR → optimise → CIL → assembly → simulator.
"""

from __future__ import annotations

from twig_clr_compiler import compile_source, run_source


def _run(source: str) -> int:
    """Compile + run + extract the integer return value.

    The CLR simulator returns a typed ``CliValue``; we unbox the
    inner Python value (an ``int`` for arithmetic programs).
    """
    result = run_source(source)
    rv = result.vm_result.return_value
    if rv is None:
        return 0
    return int(rv.value)


# ---------------------------------------------------------------------------
# Compile-only smoke tests (don't run, just confirm assembly bytes emerge)
# ---------------------------------------------------------------------------


def test_compile_produces_assembly_bytes() -> None:
    result = compile_source("(+ 1 2)")
    assert isinstance(result.assembly_bytes, bytes)
    assert len(result.assembly_bytes) > 0


def test_compile_decodes_back_to_pe_file() -> None:
    """Assembly bytes round-trip through ``decode_clr_pe_file``."""
    result = compile_source("42")
    assert result.decoded_assembly is not None


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


def test_integer_literal() -> None:
    assert _run("42") == 42


def test_addition() -> None:
    assert _run("(+ 1 2)") == 3


def test_subtraction() -> None:
    assert _run("(- 10 3)") == 7


def test_multiplication() -> None:
    assert _run("(* 6 7)") == 42


def test_division_truncates() -> None:
    assert _run("(/ 20 6)") == 3


def test_nested_arithmetic() -> None:
    assert _run("(+ (* 3 4) (- 10 5))") == 17


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------


def test_if_taken() -> None:
    assert _run("(if (= 1 1) 100 200)") == 100


def test_if_not_taken() -> None:
    assert _run("(if (= 1 2) 100 200)") == 200


def test_let_binding() -> None:
    assert _run("(let ((x 5)) (* x x))") == 25


def test_nested_let() -> None:
    assert _run("(let ((a 1)) (let ((b 2)) (+ a b)))") == 3


def test_let_with_multiple_bindings() -> None:
    assert _run("(let ((a 10) (b 20)) (- a b))") == -10


def test_begin_returns_last() -> None:
    assert _run("(begin 1 2 3)") == 3


# ---------------------------------------------------------------------------
# Comparisons
# ---------------------------------------------------------------------------


def test_eq_true() -> None:
    assert _run("(= 1 1)") == 1


def test_eq_false() -> None:
    assert _run("(= 1 2)") == 0


def test_lt_true() -> None:
    assert _run("(< 1 2)") == 1


def test_gt_true() -> None:
    assert _run("(> 2 1)") == 1
