"""IrPass Protocol — the contract every optimization pass must satisfy.

Design Rationale
----------------

Optimization passes in a compiler pipeline share a simple interface: take a
program, return a (possibly smaller or faster) program. Python's ``Protocol``
mechanism (PEP 544) lets us express this contract without requiring inheritance.

Any class that has:

  1. A ``name`` property returning a string
  2. A ``run(program) -> IrProgram`` method

…automatically satisfies ``IrPass``. This is called **structural subtyping**
(or "duck typing with types"). You never need to write ``class MyPass(IrPass)``
— just implement the two members and the type checker is satisfied.

Why Protocols instead of ABCs?
--------------------------------

Abstract Base Classes (ABCs) require inheritance, which creates tight coupling.
If you want to wrap a third-party class or a lambda as a pass, ABCs get awkward.
Protocols side-step this: the optimizer never imports the pass modules at all —
it just calls ``pass.name`` and ``pass.run()``. Any object that responds to those
two calls works.

This is the same design as Go's ``io.Reader`` (just have a ``Read`` method) or
Rust's traits.

Purity Requirement
------------------

Every pass MUST be pure: it never mutates the input ``IrProgram``. It always
returns a *new* ``IrProgram`` (or the same object if no changes were made).
This rule has three benefits:

  1. **Composability** — you can chain passes without worrying that pass A's
     output is modified by pass B before pass A finishes using it.
  2. **Testability** — you can run a pass on a program and assert the original
     is unchanged.
  3. **Reproducibility** — running the same pass twice on the same input always
     gives the same output.

Example
-------

::

    from compiler_ir import IrProgram
    from ir_optimizer.protocol import IrPass

    class NoOpPass:
        @property
        def name(self) -> str:
            return "NoOpPass"

        def run(self, program: IrProgram) -> IrProgram:
            return program  # no changes

    # Type checker accepts this as an IrPass even without explicit inheritance:
    pass_: IrPass = NoOpPass()
"""

from __future__ import annotations

from typing import Protocol

from compiler_ir import IrProgram


class IrPass(Protocol):
    """A single IR optimization pass.

    Structural protocol — any class with a ``name`` property and a ``run()``
    method that takes and returns an ``IrProgram`` satisfies this interface.
    No explicit inheritance from ``IrPass`` is required.

    Passes must be **pure**: they never mutate the input program. They return
    a new ``IrProgram``. If no changes are needed, the pass may return the
    input program object unchanged.

    This makes passes independently composable and testable:

    ::

        result_a = pass_a.run(program)
        result_b = pass_b.run(program)  # same input, unaffected by pass_a

    Attributes:
        name: Human-readable pass name for diagnostics (e.g. ``'DeadCodeEliminator'``).

    Methods:
        run: Apply this pass to a program, returning the optimized result.
    """

    @property
    def name(self) -> str:
        """Human-readable pass name for debug output.

        Used by ``IrOptimizer`` to populate ``OptimizationResult.passes_run``.
        Should be a short CamelCase name like ``'DeadCodeEliminator'``.

        Returns:
            A non-empty string identifying this pass.
        """
        ...

    def run(self, program: IrProgram) -> IrProgram:
        """Run this optimization pass on the program.

        Args:
            program: The IR program to optimize. This object must not be
                     mutated by the pass implementation.

        Returns:
            A new ``IrProgram`` with the optimization applied, or the same
            ``program`` object if no changes were needed.
        """
        ...
