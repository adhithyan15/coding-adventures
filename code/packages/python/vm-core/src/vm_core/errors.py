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


class UncaughtConditionError(VMError):
    """Raised when a THROW propagates to the top of the call stack with no handler.

    The ``throw`` opcode (VMCOND00 Layer 2) walks the static exception table of
    every active frame from innermost to outermost.  If no matching entry is
    found anywhere in the call stack, the condition is "uncaught" and the VM
    raises this error, terminating the execution.

    Attributes
    ----------
    condition:
        The condition object that was thrown but never caught.  Inspecting
        this value tells the host environment what went wrong.

    Example::

        from vm_core.errors import UncaughtConditionError
        try:
            vm.execute(module)
        except UncaughtConditionError as e:
            print(f"VM aborted: {e.condition!r}")
    """

    def __init__(self, condition: object) -> None:
        # Use a hardened repr: guest objects control __repr__ and could raise
        # or produce unboundedly large strings.  Truncate to 200 chars and fall
        # back gracefully so the error path itself never throws a secondary
        # exception that would mask this one.
        try:
            cond_repr = repr(condition)
            if len(cond_repr) > 200:
                cond_repr = cond_repr[:200] + "…"
        except Exception:  # noqa: BLE001 — __repr__ must not escape
            # Guard type().__name__ too — a metaclass with a property __name__
            # can also raise, which would re-trigger secondary exception masking.
            try:
                cond_repr = f"<{type(condition).__name__} (repr failed)>"
            except Exception:  # noqa: BLE001 — last-resort fallback
                cond_repr = "<unknown (repr and type name failed)>"
        super().__init__(f"Unhandled condition: {cond_repr}")
        self.condition = condition
