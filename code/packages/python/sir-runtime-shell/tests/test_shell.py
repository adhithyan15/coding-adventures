"""Unit tests for the SIR Python shell (``backtick``) runtime.

The commands are written with ``python -c "..."`` (via :data:`sys.executable`)
rather than ``echo`` so they behave identically on Windows ``cmd.exe`` and POSIX
``/bin/sh`` — ``echo``'s quoting and flag handling differ across those shells,
whereas a Python one-liner is deterministic everywhere.
"""

from __future__ import annotations

import sys

from coding_adventures_sir_runtime_shell import Val, backtick


def _py(code: str) -> str:
    """Build a portable ``python -c "<code>"`` command line for the host shell."""
    # Quote the interpreter path (it may contain spaces, e.g. on Windows) and
    # wrap the code in double quotes; the code itself uses only single quotes.
    return f'"{sys.executable}" -c "{code}"'


class TestBacktick:
    def test_returns_captured_stdout(self) -> None:
        # The canonical case: stdout of the command comes back as a str.
        out = backtick(_py("print(123)"))
        assert "123" in out

    def test_returns_str_type(self) -> None:
        out = backtick(_py("print('hi')"))
        assert isinstance(out, str)
        assert "hi" in out

    def test_no_trailing_newline_when_print_end_empty(self) -> None:
        # Using end='' means the only output is the bare token, exactly.
        out = backtick(_py("import sys; sys.stdout.write('exact')"))
        assert out == "exact"

    def test_multiline_output_preserved(self) -> None:
        # Two print() calls yield two lines; both must survive in order.
        out = backtick(_py("print('line1'); print('line2')"))
        assert "line1" in out
        assert "line2" in out
        assert out.index("line1") < out.index("line2")

    def test_nonzero_exit_still_returns_stdout(self) -> None:
        # check=False: a command that prints then exits non-zero still yields
        # its stdout (Ruby returns stdout regardless of $?).
        out = backtick(
            _py("import sys; sys.stdout.write('partial'); sys.exit(3)")
        )
        assert out == "partial"

    def test_nonzero_exit_with_no_stdout_returns_empty(self) -> None:
        # A failure that prints nothing to stdout yields the empty string, not
        # an exception.
        out = backtick(_py("import sys; sys.exit(5)"))
        assert out == ""

    def test_stderr_is_not_part_of_the_result(self) -> None:
        # Only stdout is returned; data written to stderr must not leak in.
        out = backtick(
            _py("import sys; sys.stderr.write('ERR'); sys.stdout.write('OUT')")
        )
        assert out == "OUT"
        assert "ERR" not in out

    def test_val_is_exported(self) -> None:
        # The SIR boundary type alias is re-exported for parity with the other
        # sir-runtime-* packages.
        assert Val is not None
