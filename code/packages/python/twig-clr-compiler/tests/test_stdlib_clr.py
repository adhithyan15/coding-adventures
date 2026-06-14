"""TW04 Phase 4g — stdlib/io on the CLR (.NET runtime).

These tests verify that Twig programs importing ``stdlib/io`` from the
bundled stdlib compile and run on the real dotnet runtime, producing
the expected output on stdout.

The headline acceptance criterion:

    (module hello (import stdlib/io))
    (stdlib/io/println 42)
    (stdlib/io/println (+ 17 25))

    → stdout: b"42\\n42\\n"

Unlike the JVM backend (which always appends the return value of the last
expression as a byte), the CLR backend uses a clean exit-code convention.
The stdlib calls write their output and the process exits cleanly with
returncode=0, leaving stdout containing only the user-visible output.

Each test creates a temporary directory with the entry module source
and relies on ``resolve_modules`` with its default ``include_stdlib=True``
to locate the bundled stdlib.

Tests skip cleanly when ``dotnet`` is not on PATH.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from twig import resolve_modules

from twig_clr_compiler import dotnet_available
from twig_clr_compiler.compiler import MultiModuleClrExecutionResult, run_modules

requires_dotnet = pytest.mark.skipif(
    not dotnet_available(),
    reason="'dotnet' binary not found on PATH",
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_module(tmp_path: Path, rel: str, contents: str) -> Path:
    path = tmp_path / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
    return path


def _run(entry: str, search_dir: Path) -> MultiModuleClrExecutionResult:
    modules = resolve_modules(entry, search_paths=[search_dir])
    return run_modules(modules, entry_module=entry)


# ---------------------------------------------------------------------------
# stdlib/io — print-int and println on CLR
# ---------------------------------------------------------------------------


@requires_dotnet
class TestStdlibIoClr:
    """End-to-end tests for stdlib/io on real ``dotnet``."""

    def test_println_42(self, tmp_path: Path) -> None:
        """``(stdlib/io/println 42)`` writes "42\\n" to stdout.

        This is the headline Phase 4g acceptance criterion — importing the
        bundled stdlib and calling println outputs the decimal integer
        followed by a newline.  The CLR backend does not append extra bytes.
        """
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.returncode == 0, (
            f"returncode {result.returncode}\n"
            f"stdout={result.stdout!r}\nstderr={result.stderr!r}"
        )
        assert result.stdout == b"42\n", (
            f"expected b'42\\n', got {result.stdout!r}"
        )

    def test_println_sum_17_25(self, tmp_path: Path) -> None:
        """``(stdlib/io/println (+ 17 25))`` writes "42\\n" — acceptance criterion."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"42\n"

    def test_println_twice(self, tmp_path: Path) -> None:
        """Calling println twice produces exactly two lines — "42\\n42\\n"."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"42\n42\n"

    def test_print_int_no_newline(self, tmp_path: Path) -> None:
        """``print-int`` does not add a newline; only the digits appear."""
        _write_module(
            tmp_path,
            "user/pint.tw",
            "(module user/pint (import stdlib/io))\n"
            "(stdlib/io/print-int 99)\n",
        )
        result = _run("user/pint", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"99"

    def test_print_int_zero(self, tmp_path: Path) -> None:
        """Zero renders as '0'."""
        _write_module(
            tmp_path,
            "user/pzero.tw",
            "(module user/pzero (import stdlib/io))\n"
            "(stdlib/io/print-int 0)\n",
        )
        result = _run("user/pzero", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"0"

    def test_println_single_digit(self, tmp_path: Path) -> None:
        """Single digit renders without extra leading zeros."""
        _write_module(
            tmp_path,
            "user/pdig.tw",
            "(module user/pdig (import stdlib/io))\n"
            "(stdlib/io/println 7)\n",
        )
        result = _run("user/pdig", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"7\n"

    def test_println_large_number(self, tmp_path: Path) -> None:
        """Multi-digit number: 1000 renders as '1000'."""
        _write_module(
            tmp_path,
            "user/plarge.tw",
            "(module user/plarge (import stdlib/io))\n"
            "(stdlib/io/println 1000)\n",
        )
        result = _run("user/plarge", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"1000\n"

    def test_newline_writes_single_lf(self, tmp_path: Path) -> None:
        """``newline`` emits exactly one newline character (ASCII 10)."""
        _write_module(
            tmp_path,
            "user/pnl.tw",
            "(module user/pnl (import stdlib/io))\n"
            "(stdlib/io/newline)\n",
        )
        result = _run("user/pnl", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"\n"

    def test_print_bool_true(self, tmp_path: Path) -> None:
        """``print-bool 1`` writes the string 'true' to stdout."""
        _write_module(
            tmp_path,
            "user/pbtrue.tw",
            "(module user/pbtrue (import stdlib/io))\n"
            "(stdlib/io/print-bool 1)\n",
        )
        result = _run("user/pbtrue", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"true"

    def test_print_bool_false(self, tmp_path: Path) -> None:
        """``print-bool 0`` writes the string 'false' to stdout."""
        _write_module(
            tmp_path,
            "user/pbfalse.tw",
            "(module user/pbfalse (import stdlib/io))\n"
            "(stdlib/io/print-bool 0)\n",
        )
        result = _run("user/pbfalse", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"false"

    def test_print_int_result_of_comparison(self, tmp_path: Path) -> None:
        """Print the boolean result (0/1) of a comparison."""
        _write_module(
            tmp_path,
            "user/pcmp.tw",
            "(module user/pcmp (import stdlib/io))\n"
            "(stdlib/io/print-int (if (< 3 5) 1 0))\n",
        )
        result = _run("user/pcmp", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"1"

    def test_println_with_define(self, tmp_path: Path) -> None:
        """User-defined function result fed to println."""
        _write_module(
            tmp_path,
            "user/pfn.tw",
            "(module user/pfn (import stdlib/io))\n"
            "(define (square x) (* x x))\n"
            "(stdlib/io/println (square 6))\n",
        )
        result = _run("user/pfn", tmp_path)
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"36\n"

    def test_returns_execution_result_type(self, tmp_path: Path) -> None:
        """``run_modules`` returns a ``MultiModuleClrExecutionResult``."""
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 1)\n",
        )
        result = _run("user/hello", tmp_path)
        assert isinstance(result, MultiModuleClrExecutionResult)


# ---------------------------------------------------------------------------
# Module order check (no dotnet needed)
# ---------------------------------------------------------------------------


class TestStdlibIoModuleOrderClr:
    """Verify the stdlib is resolved before user modules (deps-first)."""

    def test_stdlib_in_resolved_module_list(self, tmp_path: Path) -> None:
        _write_module(
            tmp_path,
            "user/check.tw",
            "(module user/check (import stdlib/io))\n"
            "(stdlib/io/println 1)\n",
        )
        modules = resolve_modules("user/check", search_paths=[tmp_path])
        names = [m.name for m in modules]
        assert "stdlib/io" in names
        assert names.index("stdlib/io") < names.index("user/check")
