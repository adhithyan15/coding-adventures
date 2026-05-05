"""VMCOND00 Layer 3 — handler chain node type.

The handler chain is a per-VM-instance ordered collection of
:class:`HandlerNode` objects maintained by the ``push_handler`` and
``pop_handler`` opcodes.  When ``signal``, ``error``, or ``warn`` executes,
it walks the chain from the most recently pushed handler to the oldest,
looking for the first node whose ``condition_type`` matches the thrown
condition.

Representation
--------------
The chain is stored as a plain Python list on :class:`vm_core.core.VMCore`
(``vm._handler_chain``).  New nodes are appended at the end; the search
direction is reversed (iterate from the END, i.e. the most recently pushed
handler first).  This avoids allocating individual linked-list node objects
and keeps the common case (empty chain) at zero cost.

Design notes
------------
- ``condition_type`` uses the same string matching semantics as
  :class:`~interpreter_ir.exception_table.ExceptionTableEntry`: ``"*"``
  is the catch-all sentinel; any other string is matched against
  ``type(condition).__name__``.  Phase 4 will extend this to a full
  subtype walk using the module's condition type registry.

- ``handler_fn`` is stored as the raw runtime value from the register that
  was referenced in the ``push_handler`` instruction.  In Phase 3 the
  frontend is expected to place the IIR function *name string* in the
  register; the dispatch handler resolves it to an ``IIRFunction`` via
  ``vm._module.get_function()``.  In Phase 4 closures will also be
  supported.

- ``stack_depth`` records ``len(vm._frames)`` at the moment the handler was
  pushed.  It is not used in Phase 3 but will be needed in Phase 5 when
  ``EXIT_TO`` needs to unwind to the correct frame depth while also popping
  any handler-chain nodes that were established deeper than the exit point.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class HandlerNode:
    """One node in the VMCOND00 Layer 3 handler chain.

    Parameters
    ----------
    condition_type:
        The condition type this handler covers.  ``"*"`` matches every
        condition (catch-all).  Any other string matches when
        ``type(condition).__name__ == condition_type`` (exact name match,
        Phase 3 semantics; Phase 4 adds subtype hierarchy).
    handler_fn:
        The runtime callable stored in the register at push time.  In
        Phase 3 this is expected to be a string (the name of an IIR
        function in the current module).
    stack_depth:
        The call stack depth (``len(vm._frames)``) when this node was
        pushed.  Reserved for Phase 5 ``EXIT_TO`` stack-unwind bookkeeping.

    Example — push a catch-all handler for function ``"my_handler"``::

        from vm_core.handler_chain import HandlerNode
        vm._handler_chain.append(
            HandlerNode(condition_type="*", handler_fn="my_handler", stack_depth=1)
        )
    """

    condition_type: str
    handler_fn: object  # str (IIR fn name) in Phase 3; closure in Phase 4
    stack_depth: int
