"""DeadCodeEliminator — remove instructions that can never be reached.

The Unreachable Code Problem
-----------------------------

When a compiler generates code for a loop, it often emits:

  1. A jump to the loop exit at the top of the loop body (to check the
     condition before the first iteration).
  2. The loop body instructions.
  3. A jump back to the top of the loop at the bottom.
  4. The loop exit label.

If the compiler is not careful about ordering, or if a previous transformation
moves a jump, instructions between the jump and the next label can become
unreachable — they can never be executed because control flow bypasses them.

Example of dead code:

::

    JUMP  loop_end        ; always jumps to loop_end
    ADD_IMM v1, v1, 1    ; DEAD — CPU never executes this
    LOAD_BYTE v2, v0, v1 ; DEAD — CPU never executes this
    loop_end:             ; LIVE — can be jumped to from anywhere
    HALT                  ; LIVE — reachable via fall-through from label

After DeadCodeEliminator:

::

    JUMP  loop_end
    loop_end:
    HALT

How the Pass Works
------------------

The pass maintains a boolean ``reachable`` flag, initialized to ``True``.
It scans instructions left-to-right:

  1. If the current instruction is a ``LABEL``, set ``reachable = True``
     (labels can be jumped to from anywhere; what follows is always reachable).
  2. If ``reachable`` is True, keep the instruction.
  3. If the current instruction is ``JUMP``, ``RET``, or ``HALT``, set
     ``reachable = False`` (nothing after an unconditional branch is reachable
     until the next label).

Note: ``BRANCH_Z`` and ``BRANCH_NZ`` are *conditional* branches — the CPU
*might* fall through to the next instruction, so they do NOT set
``reachable = False``.

Why Only JUMP, RET, HALT?
--------------------------

``JUMP`` is an *unconditional* branch — control flow always transfers to the
target. ``RET`` always returns to the caller. ``HALT`` always terminates the
program. In all three cases, the next sequential instruction is provably
unreachable.

``BRANCH_Z``/``BRANCH_NZ`` are conditional — the branch may or may not be
taken. The instruction after them is always potentially reachable (fall-through
path), so we never mark them as making subsequent code unreachable.

Purity
------

This pass never mutates the input program. It builds a new ``list[IrInstruction]``
and returns a new ``IrProgram``. The original program is unchanged.
"""

from __future__ import annotations

from compiler_ir import IrInstruction, IrProgram
from compiler_ir.opcodes import IrOp

# The set of opcodes that unconditionally transfer control — anything after
# one of these is dead until the next label.
#
# Think of it like road closures:
#   - JUMP  → road diverts; all cars turn off; the straight road is closed
#   - RET   → road ends; cars exit; no one continues
#   - HALT  → road is physically removed; no one passes
#
# BRANCH_Z and BRANCH_NZ are like forks: one road may be blocked, but cars
# can still take the other (fall-through) road.
_UNCONDITIONAL_BRANCHES: frozenset[IrOp] = frozenset(
    {IrOp.JUMP, IrOp.RET, IrOp.HALT}
)


class DeadCodeEliminator:
    """Remove instructions that can never be reached.

    Scans instructions sequentially and removes those that follow an
    unconditional branch (``JUMP``, ``RET``, ``HALT``) without an intervening
    label. Labels are always kept because they may be jumped to from elsewhere
    in the program.

    This is a single-pass O(n) algorithm — no control flow graph needed.

    Example::

        # Input:
        #   JUMP  loop_end
        #   ADD_IMM v1, v1, 1   ← dead
        #   LABEL loop_end
        #   HALT
        #
        # Output:
        #   JUMP  loop_end
        #   LABEL loop_end
        #   HALT

        elim = DeadCodeEliminator()
        result = elim.run(program)
    """

    @property
    def name(self) -> str:
        """The name of this pass, used in diagnostic output.

        Returns:
            ``'DeadCodeEliminator'``
        """
        return "DeadCodeEliminator"

    def run(self, program: IrProgram) -> IrProgram:
        """Remove unreachable instructions from the program.

        Scans instruction list sequentially. Tracks whether control flow can
        reach the current instruction (``reachable`` flag). After a
        ``JUMP``/``RET``/``HALT``, marks subsequent instructions as
        unreachable until the next ``LABEL``.

        Args:
            program: The IR program to optimize. Not mutated.

        Returns:
            A new ``IrProgram`` with unreachable instructions removed.
            If no instructions are dead, the instruction list is the same
            content but in a new ``IrProgram`` object.
        """
        live_instrs: list[IrInstruction] = []

        # True when the instruction at the current position can be executed.
        # Starts True because the first instruction is always reachable.
        reachable = True

        for instr in program.instructions:
            # A label makes subsequent code reachable: any JUMP elsewhere in
            # the program can jump to this label, so what follows must be kept.
            #
            # We check for LABEL *before* the reachability gate so that a
            # label immediately after a JUMP is never discarded.
            if instr.opcode == IrOp.LABEL:
                reachable = True

            # Keep this instruction only if it's reachable.
            if reachable:
                live_instrs.append(instr)

            # After an unconditional branch, mark subsequent code as dead.
            # We do this *after* appending the branch itself (the JUMP/RET/HALT
            # is live; only what comes after it is dead).
            if instr.opcode in _UNCONDITIONAL_BRANCHES:
                reachable = False

        # Return a new IrProgram with the same data/entry but trimmed instructions.
        # data declarations and entry_label are unchanged — we only modified
        # the instruction stream.
        return IrProgram(
            instructions=live_instrs,
            data=program.data,
            entry_label=program.entry_label,
            version=program.version,
        )
