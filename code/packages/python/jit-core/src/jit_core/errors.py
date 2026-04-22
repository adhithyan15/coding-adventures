"""Exception hierarchy for jit-core."""

from __future__ import annotations


class JITError(Exception):
    """Base class for all jit-core errors."""


class DeoptimizerError(JITError):
    """Raised when a compiled function deopts too frequently.

    When ``deopt_count / exec_count > 0.1`` the JIT invalidates the compiled
    version and marks the function as ``UNSPECIALIZABLE``.  This error is
    raised internally; callers see fallback to the interpreter.
    """


class UnspecializableError(JITError):
    """Raised when compile() is called on a function marked unspecializable.

    A function becomes unspecializable after its deopt rate exceeds 10%.
    Attempting to compile it again raises this error.
    """
