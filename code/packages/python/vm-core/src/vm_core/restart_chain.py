"""VMCOND00 Layer 4 — restart chain node type.

The restart chain is a per-VM-instance ordered collection of
:class:`RestartNode` objects maintained by the ``push_restart`` and
``pop_restart`` opcodes.  When ``find_restart`` executes, it walks the
chain from the most recently pushed node to the oldest, returning the
first node whose ``name`` matches the requested symbol.

Restarts vs. Handlers (Layer 3 contrast)
-----------------------------------------
Layer 3 handlers are invoked *automatically* when a condition is signaled;
the signaling code doesn't know in advance what (if any) handler will run.

Layer 4 restarts are *explicitly chosen*: a handler discovers the available
restarts (via ``find_restart`` / ``compute_restarts``), picks one by name,
and invokes it via ``invoke_restart``.  This separation lets the handler
"advise" the error site about how to recover without the error site being
tightly coupled to the recovery strategy.

For example, a "division-by-zero" error site might establish two restarts:
  - ``use-value``  — substitute a caller-supplied value for the result
  - ``return-zero`` — return 0 unconditionally

A handler can then find ``use-value`` and invoke it with the desired
substitute, or find ``return-zero`` and invoke it without an argument.

Representation
--------------
The chain is stored as a plain Python list on :class:`vm_core.core.VMCore`
(``vm._restart_chain``).  New nodes are appended at the end; searches iterate
from the END (most recently pushed = innermost = highest priority).

Design notes
------------
- ``name`` is a plain string (the symbol name).  Two restarts with the same
  name result in the inner one shadowing the outer one — ``find_restart``
  returns the first match from the inside out.

- ``restart_fn`` is stored as the raw runtime value from the register that
  was referenced in the ``push_restart`` instruction.  In Phase 4 the
  frontend is expected to place the IIR function *name string* in the
  register; the dispatch handler resolves it to an ``IIRFunction`` via
  ``vm._module.get_function()``.

- ``stack_depth`` records ``len(vm._frames)`` at the moment the restart was
  pushed.  It is used by EXIT_TO (Layer 5) to determine which restart-chain
  nodes were established in frames being unwound: any node with
  ``stack_depth > target_depth`` is removed during the unwind.

Example — pushing a restart named ``"use-value"`` for function
``"use_value_impl"``::

    from vm_core.restart_chain import RestartNode
    vm._restart_chain.append(
        RestartNode(name="use-value", restart_fn="use_value_impl", stack_depth=2)
    )
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class RestartNode:
    """One node in the VMCOND00 Layer 4 restart chain.

    Parameters
    ----------
    name:
        The restart's symbolic name.  ``find_restart`` matches nodes by
        exact string equality.  Conventionally restart names use kebab-case
        (e.g. ``"use-value"``, ``"return-zero"``), but any non-empty string
        is valid.
    restart_fn:
        The runtime callable stored in the register at push time.  In Phase 4
        this is expected to be a string (the name of an IIR function in the
        current module).  Future phases may allow closures.
    stack_depth:
        The call stack depth (``len(vm._frames)``) when this node was pushed.
        Used by EXIT_TO to identify nodes that were established in frames being
        unwound.

    Example — creating a restart handle and invoking it::

        from vm_core.restart_chain import RestartNode
        # The dispatch handler for find_restart writes RestartNode objects
        # into registers; invoke_restart reads them back and calls restart_fn.
        node = RestartNode(name="use-value", restart_fn="my_restart", stack_depth=1)
        # node is stored in a VM register; invoke_restart pushes a call frame
        # for "my_restart" with the supplied argument.
    """

    name: str
    restart_fn: object  # str (IIR fn name) in Phase 4; closure in future phases
    stack_depth: int
