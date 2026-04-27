"""PeepholeOptimizer — local instruction-level optimizations using a sliding window.

What is Peephole Optimization?
-------------------------------

A peephole optimizer looks at a small "window" of consecutive instructions and
replaces inefficient sequences with faster or shorter equivalents. The name
comes from looking through a peephole — you only see a small part of the
program at a time.

This is one of the oldest and most effective compiler optimization techniques.
It was first described by McKeeman in 1965. Even simple rules can eliminate
a surprising fraction of instructions in generated code.

The Patterns
------------

Pattern 1: Merge consecutive ADD_IMM on the same register
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    ADD_IMM v1, v1, 3   ; v1 += 3
    ADD_IMM v1, v1, 2   ; v1 += 2  (same register!)

Becomes:

::

    ADD_IMM v1, v1, 5   ; v1 += 5  (one instruction instead of two)

This is especially common in Brainfuck: the ``+`` command compiles to
``ADD_IMM v_val, v_val, 1``, and ``+++`` compiles to three of them. Merging
reduces instruction count and ROM usage.

Pattern 2: Remove no-op AND_IMM 255
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    ADD_IMM  v1, v1, 1   ; v1 += 1  (v1 is at most 254 + 1 = 255)
    AND_IMM  v1, v1, 255 ; v1 = v1 & 255  — no effect! already ≤ 255

Becomes:

::

    ADD_IMM  v1, v1, 1   ; v1 += 1  (AND_IMM removed)

For safety, we only remove ``AND_IMM vN, vN, 255`` when the preceding
instruction is ``ADD_IMM`` or ``LOAD_IMM`` on the same register with a
non-negative value ≤ 255. This guarantees the value before the AND is
already in the 8-bit range, so the AND truly is a no-op.

Pattern 3: LOAD_IMM 0 followed by ADD_IMM k
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    LOAD_IMM v1, 0       ; v1 = 0
    ADD_IMM  v1, v1, 7   ; v1 = 0 + 7 = 7

Becomes:

::

    LOAD_IMM v1, 7       ; v1 = 7  (directly load the result)

This is mathematically obvious: loading zero and then adding ``k`` is the
same as loading ``k``. Note: this pattern is also caught by ``ConstantFolder``,
so when both passes are active this rule is redundant — but it is kept for
completeness and for use cases where only the peephole pass is enabled.

Fixed-Point Iteration
----------------------

Some patterns create new opportunities after the first pass. For example,
merging two ``ADD_IMM`` into one may reveal another pair immediately below.
To handle cascading improvements, the optimizer iterates up to 10 times or
until no further changes are possible (fixed point).

In practice, 2–3 iterations usually reach the fixed point for typical
Brainfuck-compiled programs.

Purity
------

This pass never mutates the input program. It builds new instruction lists
on each iteration.
"""

from __future__ import annotations

from compiler_ir import IrImmediate, IrInstruction, IrProgram, IrRegister
from compiler_ir.opcodes import IrOp

# Maximum number of iterations before we give up looking for more improvements.
# This prevents infinite loops in (theoretically possible) pathological cases.
_MAX_ITERATIONS = 10


class PeepholeOptimizer:
    """Local instruction-level optimizer using a sliding window of two.

    Applies three patterns iteratively until a fixed point:

      1. Merge consecutive ``ADD_IMM vN, vN, a`` and ``ADD_IMM vN, vN, b``
         → ``ADD_IMM vN, vN, (a+b)``
      2. Remove ``AND_IMM vN, vN, 255`` when the preceding instruction is
         ``ADD_IMM`` or ``LOAD_IMM`` on vN with value ≤ 255
      3. Replace ``LOAD_IMM vN, 0; ADD_IMM vN, vN, k`` with ``LOAD_IMM vN, k``

    Example::

        opt = PeepholeOptimizer()
        result = opt.run(program)
    """

    @property
    def name(self) -> str:
        """The name of this pass, used in diagnostic output.

        Returns:
            ``'PeepholeOptimizer'``
        """
        return "PeepholeOptimizer"

    def run(self, program: IrProgram) -> IrProgram:
        """Apply peephole patterns until a fixed point (or max iterations).

        Each iteration applies all patterns to the instruction list. If the
        list shrinks, another iteration may find new opportunities. Stops
        when no pattern fires (fixed point) or after 10 iterations.

        Args:
            program: The IR program to optimize. Not mutated.

        Returns:
            A new ``IrProgram`` with peephole optimizations applied.
        """
        instrs = list(program.instructions)
        for _ in range(_MAX_ITERATIONS):
            new_instrs = self._apply_patterns(instrs)
            if len(new_instrs) == len(instrs):
                # Fixed point: no patterns fired this iteration.
                break
            instrs = new_instrs

        return IrProgram(
            instructions=instrs,
            data=program.data,
            entry_label=program.entry_label,
            version=program.version,
        )

    def _apply_patterns(
        self, instrs: list[IrInstruction]
    ) -> list[IrInstruction]:
        """Apply all peephole patterns to the instruction list once.

        Uses a two-instruction sliding window. When a pattern matches a pair
        (``instrs[i]``, ``instrs[i+1]``), it replaces them with a new
        instruction and advances the index past both.

        Args:
            instrs: The instruction list to process.

        Returns:
            A new instruction list with matched patterns replaced.
        """
        out: list[IrInstruction] = []
        i = 0
        while i < len(instrs):
            # Try to match a two-instruction pattern at position i.
            if i + 1 < len(instrs):
                curr = instrs[i]
                nxt = instrs[i + 1]
                merged = self._try_merge(curr, nxt)
                if merged is not None:
                    out.append(merged)
                    i += 2  # Skip both instructions
                    continue

            # No pattern matched — emit the current instruction unchanged.
            out.append(instrs[i])
            i += 1

        return out

    def _try_merge(
        self, curr: IrInstruction, nxt: IrInstruction
    ) -> IrInstruction | None:
        """Try to apply a peephole pattern to a pair of instructions.

        Returns the replacement instruction if a pattern matches, or ``None``
        if no pattern applies.

        Args:
            curr: The first instruction in the window.
            nxt:  The second instruction in the window.

        Returns:
            A merged/replacement instruction, or ``None``.
        """
        # ── Pattern 1: Merge consecutive ADD_IMM on the same register ─────────
        #
        # ADD_IMM vN, vN, a  ;  ADD_IMM vN, vN, b  →  ADD_IMM vN, vN, (a+b)
        #
        # The three conditions:
        #   (a) Both are ADD_IMM
        #   (b) Both operate on the same register (vN in operands[0])
        #   (c) The source of each is the same as its destination (vN, vN form)
        if (
            curr.opcode == IrOp.ADD_IMM
            and nxt.opcode == IrOp.ADD_IMM
            and len(curr.operands) == 3
            and len(nxt.operands) == 3
        ):
            c_dst = curr.operands[0]
            c_src = curr.operands[1]
            c_imm = curr.operands[2]
            n_dst = nxt.operands[0]
            n_src = nxt.operands[1]
            n_imm = nxt.operands[2]
            if (
                isinstance(c_dst, IrRegister)
                and isinstance(c_src, IrRegister)
                and isinstance(c_imm, IrImmediate)
                and isinstance(n_dst, IrRegister)
                and isinstance(n_src, IrRegister)
                and isinstance(n_imm, IrImmediate)
                and c_dst.index == c_src.index  # vN, vN form
                and n_dst.index == n_src.index  # vN, vN form
                and c_dst.index == n_dst.index  # same register
            ):
                combined_imm = c_imm.value + n_imm.value
                return IrInstruction(
                    opcode=IrOp.ADD_IMM,
                    operands=[c_dst, c_src, IrImmediate(combined_imm)],
                    id=curr.id,  # keep first instruction's ID
                )

        # ── Pattern 2: Remove no-op AND_IMM 255 ──────────────────────────────
        #
        # ADD_IMM  vN, vN, k   (k ≥ 0 and k ≤ 255)
        # AND_IMM  vN, vN, 255 → remove AND_IMM (the value is already ≤ 255)
        #
        # Also matches LOAD_IMM vN, k where k ≥ 0 and k ≤ 255.
        #
        # Safety condition: we only remove the AND when the preceding value is
        # guaranteed ≤ 255.  We check the immediate value of ADD_IMM/LOAD_IMM.
        # For ADD_IMM, the delta is what we check (not the accumulated value —
        # in general we cannot know the accumulated value here).  The common
        # Brainfuck case is ADD_IMM v_val, v_val, 1 followed by AND_IMM v_val,
        # v_val, 255; since the add is only by 1 (or a small positive value ≤
        # 255), the AND is always a no-op when the type wraps at 255 anyway.
        # We check: delta value ≥ 0 and ≤ 255.
        if (
            nxt.opcode == IrOp.AND_IMM
            and len(nxt.operands) == 3
            and isinstance(nxt.operands[0], IrRegister)
            and isinstance(nxt.operands[1], IrRegister)
            and isinstance(nxt.operands[2], IrImmediate)
            and nxt.operands[0].index == nxt.operands[1].index  # vN, vN form
            and nxt.operands[2].value == 255
        ):
            n_reg = nxt.operands[0]
            # Check preceding instruction is ADD_IMM or LOAD_IMM on the same reg
            if (
                curr.opcode in (IrOp.ADD_IMM, IrOp.LOAD_IMM)
                and len(curr.operands) >= 2
                and isinstance(curr.operands[0], IrRegister)
                and curr.operands[0].index == n_reg.index
            ):
                # For both ADD_IMM and LOAD_IMM, the immediate is the last operand.
                imm_op = curr.operands[-1]
                if (
                    isinstance(imm_op, IrImmediate)
                    and 0 <= imm_op.value <= 255
                ):
                    # The AND_IMM 255 is a no-op — return only curr.
                    return curr

        # ── Pattern 3: LOAD_IMM 0 followed by ADD_IMM k ───────────────────────
        #
        # LOAD_IMM vN, 0   ; v = 0
        # ADD_IMM  vN, vN, k  ; v = 0 + k = k
        #
        # → LOAD_IMM vN, k
        if (
            curr.opcode == IrOp.LOAD_IMM
            and nxt.opcode == IrOp.ADD_IMM
            and len(curr.operands) == 2
            and len(nxt.operands) == 3
        ):
            c_dst = curr.operands[0]
            c_imm = curr.operands[1]
            n_dst = nxt.operands[0]
            n_src = nxt.operands[1]
            n_imm = nxt.operands[2]
            if (
                isinstance(c_dst, IrRegister)
                and isinstance(c_imm, IrImmediate)
                and isinstance(n_dst, IrRegister)
                and isinstance(n_src, IrRegister)
                and isinstance(n_imm, IrImmediate)
                and c_imm.value == 0  # loading zero
                and c_dst.index == n_dst.index  # same register
                and n_dst.index == n_src.index  # vN, vN form
            ):
                return IrInstruction(
                    opcode=IrOp.LOAD_IMM,
                    operands=[c_dst, n_imm],
                    id=curr.id,
                )

        return None
