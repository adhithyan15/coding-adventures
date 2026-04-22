"""Exception hierarchy for vm-core."""

from __future__ import annotations


class VMError(Exception):
    """Base class for all vm-core errors."""


class UnknownOpcodeError(VMError):
    """Raised when the dispatch table has no handler for an opcode."""


class FrameOverflowError(VMError):
    """Raised when a CALL instruction would exceed the maximum frame depth."""


class UndefinedVariableError(VMError):
    """Raised when an instruction references a variable name not in scope."""


class VMInterrupt(VMError):
    """Raised by the dispatch loop when vm.interrupt() is called.

    Caught by VMCore.execute() and reported as a KeyboardInterrupt-equivalent.
    """
