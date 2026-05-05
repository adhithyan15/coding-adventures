"""VMCOND00 Layer 5 — exit-point chain node type.

The exit-point chain is a per-VM-instance ordered collection of
:class:`ExitPointNode` objects maintained by the ``establish_exit`` opcode.
Any code within the *dynamic extent* of an ``establish_exit`` can call
``exit_to`` with the matching tag to perform a non-local transfer: the call
stack, handler chain, and restart chain are all unwound to the depth recorded
at ``establish_exit`` time, a value is delivered into the exit point's result
register, and execution resumes at the ``resume_ip`` instruction.

Dynamic extent
--------------
The dynamic extent of an ``establish_exit`` starts at the ``establish_exit``
instruction itself and ends when:

1. **``exit_to`` fires** — the most common case.  EXIT_TO finds the matching
   exit point, unwinds all chains, delivers the value, and jumps to
   ``resume_ip``.  The exit point node is removed from the chain.

2. **Normal fallthrough** — the code within the guarded region completes
   without calling ``exit_to``.  In this case the exit point node is left
   on the chain until the enclosing frame pops (via ``ret``/``ret_void``).
   The :func:`vm_core.dispatch.handle_ret` / :func:`handle_ret_void` handlers
   automatically clean up exit-point nodes whose ``frame_depth`` matches the
   depth of the frame being popped, so no cleanup leak occurs.

Contrast with Layer 2 (THROW)
-------------------------------
- ``THROW`` walks a *static* exception table attached to each function.
- ``EXIT_TO`` walks a *dynamic* chain that reflects the live runtime state.

A restart body (Layer 4) that wants to transfer control non-locally calls
``exit_to``; the matching ``establish_exit`` in the outer code receives the
restart's return value and continues from the labelled point.

Representation
--------------
The chain is a plain Python list on :class:`vm_core.core.VMCore`
(``vm._exit_point_chain``).  New nodes are appended at the end; searches
iterate from the END (most recently pushed = innermost) to find the correct
dynamic extent.

Design notes
------------
- ``tag`` is a plain string (symbol name).  The same scoping rules as
  CATCH / THROW in Common Lisp: the innermost matching exit point wins.

- ``result_reg`` is the *name* of the register in the frame at ``frame_depth``
  that will receive the exit value.  It is a string (IIR register name) rather
  than a register index because :meth:`~vm_core.frame.VMFrame.assign` takes
  a name.

- ``resume_ip`` is the resolved instruction index (not a label name) so that
  EXIT_TO can set ``vm._frames[-1].ip`` directly without a second label lookup.

- ``frame_depth`` is ``len(vm._frames)`` at ``establish_exit`` time, which is
  also the target depth for the unwind.  After unwinding,
  ``vm._frames[-1]`` is exactly the frame that executed ``establish_exit``.

Example::

    from vm_core.exit_chain import ExitPointNode
    # establish_exit "done", "r", after_ip=10 in frame at depth 1
    node = ExitPointNode(
        tag="done",
        result_reg="r",
        resume_ip=10,
        frame_depth=1,
    )
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class ExitPointNode:
    """One node in the VMCOND00 Layer 5 exit-point chain.

    Parameters
    ----------
    tag:
        The exit-point tag string.  ``exit_to`` matches nodes by exact
        string equality searching from the most recently pushed to the oldest.
        Conventionally tags use kebab-case (e.g. ``"done"``, ``"abort"``).
    result_reg:
        Name of the register in the frame at ``frame_depth`` that receives
        the value passed to ``exit_to``.  The register must exist in that
        frame's register file.
    resume_ip:
        The instruction index within the function at ``frame_depth`` to which
        the instruction pointer is set after EXIT_TO delivers the value.  This
        is the resolved index of the ``after`` label supplied to
        ``establish_exit``.
    frame_depth:
        ``len(vm._frames)`` at the time ``establish_exit`` ran.  EXIT_TO
        unwinds the call stack to exactly this depth before delivering the
        value and jumping.

    Example::

        node = ExitPointNode(
            tag="done",
            result_reg="result",
            resume_ip=15,
            frame_depth=2,
        )
        # After EXIT_TO("done", value):
        # - Frames above depth 2 are popped.
        # - vm._frames[-1].assign("result", value)
        # - vm._frames[-1].ip = 15
    """

    tag: str
    result_reg: str | None  # None means EXIT_TO discards the value
    resume_ip: int
    frame_depth: int
