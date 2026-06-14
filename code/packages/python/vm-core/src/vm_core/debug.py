"""Debug hooks and step-mode control for vm-core (LANG06).

This module defines the interface that a debug adapter implements to observe
and control a running ``VMCore``.  The design follows the same pattern as the
JVM's JVMTI, V8's inspector protocol, and CPython's sys.settrace — the VM
calls registered callbacks at specific events, and the adapter responds by
calling back into the VM to continue, step, or inspect.

How it fits into the pipeline
------------------------------

    Tetrad source
      → compiler  → IIRModule + DebugSidecar
      → VMCore (with DebugHooks attached)
          on_instruction → hook fires for every IIR instruction
          on_call        → hook fires before entering a callee frame
          on_return      → hook fires after a frame returns
          on_exception   → hook fires on unhandled errors
      → DebugSidecarReader translates frame.ip → (file, line, col)

The adapter uses the sidecar reader to answer:
    "The VM paused at frame.ip=14 in function 'fibonacci' — what source line?"
    DebugSidecarReader.lookup("fibonacci", 14) → SourceLocation("fib.tetrad", 3, 5)

And to set breakpoints from source positions:
    "Break at fib.tetrad line 7."
    DebugSidecarReader.find_instr("fib.tetrad", 7) → 21
    vm.set_breakpoint(21, "fibonacci")

Step modes
----------
``StepMode`` is an enum that the dispatch loop consults after ``on_instruction``
returns to decide when to pause next.  The adapter sets it by calling one of
the step methods on ``VMCore``::

    vm.step_in()   # pause at the very next IIR instruction dispatched
    vm.step_over() # pause at the next instruction in the current or an outer frame
    vm.step_out()  # pause at the return site of the current frame

Overhead when not debugging
----------------------------
When no hooks are attached (``vm._debug_hooks is None``), the dispatch loop
skips the entire ``_check_debug_pause`` block with a single branch.  There is
zero per-instruction overhead on the normal execution path.

Usage example
-------------
::

    class MyAdapter(DebugHooks):
        def __init__(self, reader: DebugSidecarReader) -> None:
            self.reader = reader
            self.events: list[tuple[str, int]] = []

        def on_instruction(self, frame, instr) -> None:
            loc = self.reader.lookup(frame.fn.name, frame.ip - 1)
            self.events.append((frame.fn.name, frame.ip - 1))

    adapter = MyAdapter(reader)
    vm = VMCore()
    vm.attach_debug_hooks(adapter)
    vm.set_breakpoint(14, "fibonacci")
    vm.execute(module)
    # adapter.events contains every (fn_name, ip) pair where the VM paused
"""

from __future__ import annotations

from enum import Enum, auto
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from interpreter_ir import IIRInstr
    from interpreter_ir.function import IIRFunction
    from vm_core.frame import VMFrame


class StepMode(Enum):
    """The stepping granularity the dispatch loop should use after a pause.

    The adapter sets this by calling the corresponding method on ``VMCore``::

        vm.step_in()    # → StepMode.IN
        vm.step_over()  # → StepMode.OVER
        vm.step_out()   # → StepMode.OUT
        vm.continue_()  # → clears step mode

    Semantics
    ---------
    IN:
        Pause before the very next IIR instruction dispatched, regardless of
        what function it belongs to.  This is the finest granularity — it
        steps into called functions.

    OVER:
        Pause before the next IIR instruction whose frame depth is ≤ the
        depth at which the step was requested.  This skips the internals of
        any function called between now and the next same-level instruction.

    OUT:
        Pause after the current frame returns to its caller.  Implemented by
        the ``on_return`` callback: when it fires, the adapter clears the
        step mode and signals the VM to pause before the instruction that
        follows the return site in the caller.
    """

    IN = auto()
    """Step into called functions — pause at every IIR instruction."""

    OVER = auto()
    """Step over called functions — pause at same or outer frame depth."""

    OUT = auto()
    """Run until current frame returns, then pause at the return site."""


class DebugHooks:
    """Callbacks that a debug adapter registers with ``VMCore``.

    Subclass this and override the methods you need.  The default
    implementations are all no-ops so partial adapters work without inheriting
    boilerplate.

    Thread safety
    -------------
    These callbacks fire on the VM's execution thread.  If the adapter needs
    to communicate with a separate UI thread (e.g. a DAP server), it is the
    adapter's responsibility to marshal events across the thread boundary.

    Calling vm.pause() / vm.step_*() / vm.continue_() from inside a hook
    is safe — they only mutate ``vm._paused`` and ``vm._step_mode``, which
    the dispatch loop reads after the hook returns.

    Example::

        class PrintAdapter(DebugHooks):
            def on_instruction(self, frame, instr):
                print(f"  [{frame.fn.name}:{frame.ip - 1}] {instr.op}")

            def on_call(self, caller, callee):
                print(f"→ call {callee.name}")

            def on_return(self, frame, return_value):
                print(f"← return {return_value!r} from {frame.fn.name}")
    """

    def on_instruction(self, frame: "VMFrame", instr: "IIRInstr") -> None:
        """Called before an IIR instruction is dispatched (when the VM pauses).

        This fires only when the VM is about to pause — either because a
        breakpoint was hit or because a step mode requested it.  It does NOT
        fire for every instruction when the VM is running freely.

        The adapter can inspect the frame, call ``vm.call_stack()``, resolve
        the IP through a ``DebugSidecarReader``, and then choose what to do
        next by calling one of the step/continue methods on the VM.

        Parameters
        ----------
        frame:
            The current frame (top of the call stack).  ``frame.ip`` has
            already been advanced past this instruction by the dispatch loop,
            so the instruction's own index is ``frame.ip - 1``.
        instr:
            The IIR instruction about to be dispatched.
        """

    def on_call(self, caller: "VMFrame", callee: "IIRFunction") -> None:
        """Called when a CALL instruction pushes a new frame.

        Fires *before* the callee frame is pushed, so ``caller`` is still
        the top of the frame stack.  The adapter can record the call site
        for stack-trace display.

        Parameters
        ----------
        caller:
            The frame that issued the CALL.
        callee:
            The ``IIRFunction`` about to be entered.
        """

    def on_return(self, frame: "VMFrame", return_value: Any) -> None:
        """Called when a RET or RET_VOID instruction pops a frame.

        Fires *after* the frame is popped, so the frame passed in is the
        one that just returned (it is no longer on the VM's stack).  Used
        by ``StepMode.OUT`` to detect when to pause.

        Parameters
        ----------
        frame:
            The frame that just returned.
        return_value:
            The value returned by the frame (``None`` for void returns).
        """

    def on_exception(self, frame: "VMFrame", error: Exception) -> None:
        """Called when an unhandled exception is raised during execution.

        The VM will re-raise the exception after this hook returns.  The
        adapter can inspect the frame to show a post-mortem stack trace.

        Parameters
        ----------
        frame:
            The frame active when the exception occurred.
        error:
            The exception that was raised.
        """
