"""TW04 Phase 4g — stdlib/io on the JVM.

These tests verify that Twig programs importing ``stdlib/io`` from the
bundled stdlib compile and run on the real JVM, producing the expected
output on stdout.

JVM output convention
---------------------
The JVM backend's ``_start`` region always writes the *return value* of
the last top-level expression as a raw byte via ``SYSCALL 1`` after all
user-visible output has been produced.  This means that calling a stdlib
function like ``(stdlib/io/println 42)`` produces TWO writes:

1. The output emitted by ``println`` itself — ``b"42\\n"``
2. The extra byte from ``_start``'s final SYSCALL — ``println`` returns
   the return value of ``(host/write-byte 10)``, which is 10 (decimal),
   so a newline character ``b"\\n"`` is appended.

All expected values in these tests account for this extra trailing byte.

The headline acceptance criterion (with JVM trailing byte accounted for):

    (module hello (import stdlib/io))
    (stdlib/io/println 42)
    (stdlib/io/println (+ 17 25))

    → stdout: b"42\\n42\\n\\n"
              (two printlns produce 42\\n42\\n, then the JVM appends \\n)

Each test creates a temporary directory with the entry module source
and relies on ``resolve_modules`` with its default ``include_stdlib=True``
to locate the bundled stdlib.

Tests skip cleanly when ``java`` is not on PATH.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from twig import resolve_modules

from twig_jvm_compiler import java_available, run_modules
from twig_jvm_compiler.compiler import MultiModuleExecutionResult

requires_java = pytest.mark.skipif(
    not java_available(),
    reason="'java' binary not found on PATH",
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_module(tmp_path: Path, rel: str, contents: str) -> Path:
    path = tmp_path / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
    return path


def _run(entry: str, search_dir: Path) -> MultiModuleExecutionResult:
    modules = resolve_modules(entry, search_paths=[search_dir])
    return run_modules(modules, entry_module=entry)


# ---------------------------------------------------------------------------
# stdlib/io — print-int and println
# ---------------------------------------------------------------------------


@requires_java
class TestStdlibIoJvm:
    """End-to-end tests for stdlib/io on real ``java``."""

    def test_println_42(self, tmp_path: Path) -> None:
        """``(stdlib/io/println 42)`` writes "42\\n" then \\n (JVM trailing byte).

        This is the headline Phase 4g acceptance criterion — importing the
        bundled stdlib and calling println outputs the decimal integer
        followed by a newline.  The JVM backend appends an extra \\n (the
        return value of the last host/write-byte call = 10).
        """
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.exit_code == 0, (
            f"exit code {result.exit_code}\n"
            f"stdout={result.stdout!r}\nstderr={result.stderr!r}"
        )
        # "42\n" from println + "\n" from JVM _start SYSCALL (return value = 10)
        assert result.stdout == b"42\n\n", (
            f"expected b'42\\n\\n', got {result.stdout!r}"
        )

    def test_println_sum_17_25(self, tmp_path: Path) -> None:
        """``(stdlib/io/println (+ 17 25))`` writes "42\\n\\n".

        The full Phase 4g acceptance criterion: arithmetic result fed to
        println.  JVM appends the return value (10 = \\n).
        """
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.exit_code == 0, result.stderr
        assert result.stdout == b"42\n\n"

    def test_println_twice(self, tmp_path: Path) -> None:
        """Calling println twice produces two lines plus JVM trailing byte.

        The spec example calls println twice.  The second println's return
        value is appended once by the JVM _start SYSCALL.
        """
        _write_module(
            tmp_path,
            "user/hello.tw",
            "(module user/hello (import stdlib/io))\n"
            "(stdlib/io/println 42)\n"
            "(stdlib/io/println (+ 17 25))\n",
        )
        result = _run("user/hello", tmp_path)
        assert result.exit_code == 0, result.stderr
        # First println: "42\n", second println: "42\n", JVM appends "\n"
        assert result.stdout == b"42\n42\n\n"

    def test_print_int_no_newline(self, tmp_path: Path) -> None:
        """``print-int`` does not add a newline.

        For 99 (two digits): print-int writes '9' then '9'.  The last
        host/write-byte call returns 57 (ASCII '9'), so the JVM appends
        byte 57 = '9' as well → stdout is b'999'.
        """
        _write_module(
            tmp_path,
            "user/pint.tw",
            "(module user/pint (import stdlib/io))\n"
            "(stdlib/io/print-int 99)\n",
        )
        result = _run("user/pint", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "99" from print-int + "9" from JVM SYSCALL (return = 57 = '9')
        assert result.stdout == b"999"

    def test_print_int_zero(self, tmp_path: Path) -> None:
        """Zero renders as '0'.  JVM appends byte 48 ('0') as the return value."""
        _write_module(
            tmp_path,
            "user/pzero.tw",
            "(module user/pzero (import stdlib/io))\n"
            "(stdlib/io/print-int 0)\n",
        )
        result = _run("user/pzero", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "0" from print-int + "0" from JVM SYSCALL (return = 48 = '0')
        assert result.stdout == b"00"

    def test_println_single_digit(self, tmp_path: Path) -> None:
        """Single digit renders correctly (no leading zeros)."""
        _write_module(
            tmp_path,
            "user/pdig.tw",
            "(module user/pdig (import stdlib/io))\n"
            "(stdlib/io/println 7)\n",
        )
        result = _run("user/pdig", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "7\n" from println + "\n" from JVM SYSCALL (return = 10)
        assert result.stdout == b"7\n\n"

    def test_println_large_number(self, tmp_path: Path) -> None:
        """Multi-digit number: 1000 renders as '1000'."""
        _write_module(
            tmp_path,
            "user/plarge.tw",
            "(module user/plarge (import stdlib/io))\n"
            "(stdlib/io/println 1000)\n",
        )
        result = _run("user/plarge", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "1000\n" from println + "\n" from JVM SYSCALL
        assert result.stdout == b"1000\n\n"

    def test_newline_writes_single_lf(self, tmp_path: Path) -> None:
        """``newline`` emits a LF.  JVM appends a second LF (return value = 10)."""
        _write_module(
            tmp_path,
            "user/pnl.tw",
            "(module user/pnl (import stdlib/io))\n"
            "(stdlib/io/newline)\n",
        )
        result = _run("user/pnl", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "\n" from newline + "\n" from JVM SYSCALL
        assert result.stdout == b"\n\n"

    def test_print_bool_true(self, tmp_path: Path) -> None:
        """``print-bool 1`` writes 'true', JVM appends 'e' (return = 101)."""
        _write_module(
            tmp_path,
            "user/pbtrue.tw",
            "(module user/pbtrue (import stdlib/io))\n"
            "(stdlib/io/print-bool 1)\n",
        )
        result = _run("user/pbtrue", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "true" + JVM trailing byte (write-byte returns 101='e')
        assert result.stdout == b"truee"

    def test_print_bool_false(self, tmp_path: Path) -> None:
        """``print-bool 0`` writes 'false', JVM appends 'e' (return = 101)."""
        _write_module(
            tmp_path,
            "user/pbfalse.tw",
            "(module user/pbfalse (import stdlib/io))\n"
            "(stdlib/io/print-bool 0)\n",
        )
        result = _run("user/pbfalse", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "false" + JVM trailing byte (write-byte returns 101='e')
        assert result.stdout == b"falsee"

    def test_print_int_result_of_comparison(self, tmp_path: Path) -> None:
        """Print the boolean result (0/1) of a comparison.

        ``(< 3 5)`` returns 1 (truthy).  ``print-int 1`` writes '1'
        (ASCII 49).  JVM appends byte 49 = '1'.
        """
        _write_module(
            tmp_path,
            "user/pcmp.tw",
            "(module user/pcmp (import stdlib/io))\n"
            "(stdlib/io/print-int (if (< 3 5) 1 0))\n",
        )
        result = _run("user/pcmp", tmp_path)
        assert result.exit_code == 0, result.stderr
        # "1" from print-int + "1" from JVM SYSCALL (return = 49 = '1')
        assert result.stdout == b"11"

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
        assert result.exit_code == 0, result.stderr
        # "36\n" from println + "\n" from JVM SYSCALL
        assert result.stdout == b"36\n\n"


# ---------------------------------------------------------------------------
# stdlib/io resolves alongside user modules (module order check)
# ---------------------------------------------------------------------------


@requires_java
class TestStdlibIoModuleOrder:
    """Verify that the stdlib is resolved before user modules (deps-first)."""

    def test_stdlib_in_resolved_module_list(self, tmp_path: Path) -> None:
        """``resolve_modules`` with ``include_stdlib=True`` includes stdlib/io."""
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
