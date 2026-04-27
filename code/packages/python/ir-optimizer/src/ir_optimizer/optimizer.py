"""IrOptimizer — chains multiple IrPass instances into a single pipeline.

The Pipeline Model
------------------

A compiler optimizer is essentially a function composition:

    optimized = pass_n( pass_{n-1}( ... pass_1(program) ... ) )

Each pass receives the output of the previous pass. Passes are applied in
declaration order, left-to-right. This sequencing matters: for example,
``DeadCodeEliminator`` should run before ``ConstantFolder`` so that the folder
never wastes time on dead instructions.

The default pipeline is:

    DeadCodeEliminator → ConstantFolder → PeepholeOptimizer

This ordering is deliberate:

  1. **DeadCodeEliminator first** — removes unreachable instructions. This
     reduces the instruction count before the folding pass, which means fewer
     patterns to scan.
  2. **ConstantFolder second** — merges ``LOAD_IMM + ADD_IMM`` or
     ``LOAD_IMM + AND_IMM`` sequences into a single ``LOAD_IMM``. This creates
     new opportunities for the peephole pass (e.g., a ``LOAD_IMM 0`` followed
     by ``ADD_IMM`` that is now visible only after folding).
  3. **PeepholeOptimizer last** — applies small local transformations (merge
     consecutive ``ADD_IMM``, remove no-op ``AND_IMM 255``, fold ``LOAD_IMM 0
     + ADD_IMM`` into a single ``LOAD_IMM``). This pass can iterate to a fixed
     point so it catches cascading improvements.

OptimizationResult
------------------

After running the pipeline, ``IrOptimizer.optimize()`` returns an
``OptimizationResult`` that bundles:

  - The final optimized ``IrProgram``
  - The list of pass names that ran (for logging/debugging)
  - The instruction count before and after optimization

The ``instructions_eliminated`` property is convenient shorthand:

::

    result = optimizer.optimize(program)
    print(f"Eliminated {result.instructions_eliminated} instructions")
    print(f"Passes: {result.passes_run}")

Why a Separate Result Object?
------------------------------

Returning just the optimized program would make it hard for the caller to
know whether optimization had any effect. Wrapping the program in a result
object lets the backend log useful diagnostics without extra work:

::

    result = IrOptimizer.default_passes().optimize(program)
    if result.instructions_eliminated > 0:
        logger.info(
            "Optimizer: %d → %d instructions (%d eliminated, passes: %s)",
            result.instructions_before,
            result.instructions_after,
            result.instructions_eliminated,
            ", ".join(result.passes_run),
        )

On the Intel 4004 (4 KB ROM), every eliminated instruction is precious —
the Busicom calculator had to fit an entire calculator program into 4096 bytes.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from compiler_ir import IrProgram

from ir_optimizer.protocol import IrPass


@dataclass
class OptimizationResult:
    """The result of running the optimizer pipeline.

    Bundles the optimized program with diagnostic information about what the
    optimizer did. Useful for logging, testing, and debugging.

    Attributes:
        program:              The optimized ``IrProgram``.
        passes_run:           Names of every pass that ran, in order.
        instructions_before:  Instruction count before optimization.
        instructions_after:   Instruction count after optimization.

    Example::

        result = IrOptimizer.default_passes().optimize(prog)
        print(result.instructions_eliminated)  # e.g., 3
        print(result.passes_run)
        # ['DeadCodeEliminator', 'ConstantFolder', 'PeepholeOptimizer']
    """

    program: IrProgram
    passes_run: list[str] = field(default_factory=list)
    instructions_before: int = 0
    instructions_after: int = 0

    @property
    def instructions_eliminated(self) -> int:
        """Number of instructions removed by the optimizer.

        Equal to ``instructions_before - instructions_after``. May be zero if
        no instructions were removed, or negative if a pass somehow added
        instructions (unusual but possible for instrumentation passes).

        Returns:
            The net instruction reduction.
        """
        return self.instructions_before - self.instructions_after


class IrOptimizer:
    """Chains multiple ``IrPass`` instances into a single optimization pipeline.

    Each pass in the list is run in order. The output of pass N is the input
    to pass N+1. The final output becomes ``OptimizationResult.program``.

    All passes must be pure (no mutation of their inputs). The optimizer does
    not enforce this, but tests should catch violations.

    Example::

        from ir_optimizer import IrOptimizer
        from ir_optimizer.passes import DeadCodeEliminator, ConstantFolder

        optimizer = IrOptimizer([DeadCodeEliminator(), ConstantFolder()])
        result = optimizer.optimize(program)
        print(result.instructions_eliminated)

    For the standard three-pass pipeline, use the factory:

    ::

        result = IrOptimizer.default_passes().optimize(program)
    """

    def __init__(self, passes: list[IrPass]) -> None:
        """Create an optimizer with the given list of passes.

        Args:
            passes: The passes to run, in order. An empty list creates a
                    no-op optimizer (useful for testing without optimization).
        """
        self._passes = passes

    def optimize(self, program: IrProgram) -> OptimizationResult:
        """Run all passes in order and return the result.

        Passes are applied sequentially: the output of each pass feeds the
        next. The ``instructions_before`` count is captured before any pass
        runs; ``instructions_after`` is captured after all passes complete.

        Args:
            program: The IR program to optimize.

        Returns:
            An ``OptimizationResult`` with the final program and diagnostics.

        Example::

            result = optimizer.optimize(program)
            assert result.instructions_after <= result.instructions_before
        """
        instructions_before = len(program.instructions)
        passes_run: list[str] = []

        current = program
        for pass_ in self._passes:
            current = pass_.run(current)
            passes_run.append(pass_.name)

        return OptimizationResult(
            program=current,
            passes_run=passes_run,
            instructions_before=instructions_before,
            instructions_after=len(current.instructions),
        )

    @classmethod
    def default_passes(cls) -> IrOptimizer:
        """Create an optimizer with the standard three-pass pipeline.

        The pipeline is:

          DeadCodeEliminator → ConstantFolder → PeepholeOptimizer

        This ordering maximizes effectiveness:
          - DCE removes dead code first, reducing work for later passes
          - CF folds constants, creating new peephole opportunities
          - PH cleans up the resulting instruction stream

        Returns:
            An ``IrOptimizer`` configured with all three default passes.

        Example::

            result = IrOptimizer.default_passes().optimize(program)
        """
        # Import here to avoid circular dependencies at module load time.
        # The passes module imports from protocol, which imports IrProgram.
        from ir_optimizer.passes import (  # noqa: PLC0415
            ConstantFolder,
            DeadCodeEliminator,
            PeepholeOptimizer,
        )

        return cls([DeadCodeEliminator(), ConstantFolder(), PeepholeOptimizer()])

    @classmethod
    def no_op(cls) -> IrOptimizer:
        """Create an optimizer that makes no changes.

        Useful for testing backends without any optimization applied, or for
        establishing a baseline instruction count before optimization.

        Returns:
            An ``IrOptimizer`` with an empty pass list.

        Example::

            result = IrOptimizer.no_op().optimize(program)
            assert result.instructions_eliminated == 0
        """
        return cls([])
