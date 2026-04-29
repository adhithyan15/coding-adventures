"""End-to-end Twig source → real ``dotnet`` execution tests.

These tests require ``dotnet`` on PATH.  They are the headline
proof that the Twig → CLR pipeline works end-to-end on real
.NET 9.0+ — completing the Twig real-runtime trilogy alongside
JVM and BEAM.

Note on the result channel
==========================

For v1 the program's last expression value flows to the **process
exit code** (matching how a C# ``static int Main()`` program works).
That keeps the v1 surface tiny — no stdout / Console.WriteLine
needed.  v2 adds explicit I/O.

Exit codes are 32-bit signed integers, so we limit our v1 tests to
small positive results.
"""

from __future__ import annotations

import pytest

from twig_clr_compiler import dotnet_available, run_source

requires_dotnet = pytest.mark.skipif(
    not dotnet_available(),
    reason="dotnet not on PATH",
)


@requires_dotnet
def test_addition() -> None:
    """``(+ 1 2)`` exits 3 from real dotnet."""
    result = run_source("(+ 1 2)", assembly_name="ClrAdd")
    assert result.returncode == 3, (
        f"dotnet rejected the assembly or returned wrong code:\n"
        f"  exit={result.returncode}\n"
        f"  stderr={result.stderr!r}"
    )


@requires_dotnet
def test_multiplication() -> None:
    """``(* 6 7)`` exits 42."""
    result = run_source("(* 6 7)", assembly_name="ClrMul")
    assert result.returncode == 42, result.stderr


@requires_dotnet
def test_subtraction() -> None:
    """``(- 10 3)`` exits 7."""
    result = run_source("(- 10 3)", assembly_name="ClrSub")
    assert result.returncode == 7, result.stderr


@requires_dotnet
def test_division() -> None:
    """``(/ 10 2)`` exits 5."""
    result = run_source("(/ 10 2)", assembly_name="ClrDiv")
    assert result.returncode == 5, result.stderr


@requires_dotnet
def test_let_binding() -> None:
    """``(let ((x 5)) (* x x))`` exits 25."""
    result = run_source("(let ((x 5)) (* x x))", assembly_name="ClrLet")
    assert result.returncode == 25, result.stderr


@requires_dotnet
def test_nested_arithmetic() -> None:
    """``(+ (* 6 7) (* 2 3))`` exits 48."""
    result = run_source(
        "(+ (* 6 7) (* 2 3))", assembly_name="ClrNested"
    )
    assert result.returncode == 48, result.stderr
