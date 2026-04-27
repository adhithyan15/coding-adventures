"""ConstantFolder — fold constant expressions known at compile time.

What is Constant Folding?
--------------------------

Constant folding replaces expressions whose operands are all constants with
their computed result. Instead of emitting instructions to compute ``5 + 3``
at runtime, the compiler can compute ``8`` at compile time and emit a single
``LOAD_IMM v1, 8``.

In the general case, a full constant folding pass requires tracking the known
values of all registers throughout the program — a *data-flow analysis*. This
implementation is simpler: we track a "pending load" for each register (the
value most recently loaded into it via ``LOAD_IMM``) and fold only when the
next write to that register is an arithmetic immediate operation.

Folded Patterns
---------------

Pattern 1: LOAD_IMM followed by ADD_IMM
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    LOAD_IMM  v1, 5     ; v1 = 5 (known at compile time)
    ADD_IMM   v1, v1, 3 ; v1 = v1 + 3 = 8 (also known at compile time!)

Becomes:

::

    LOAD_IMM  v1, 8     ; v1 = 8 (folded — one instruction instead of two)

This pattern arises frequently in Brainfuck compilation. The Brainfuck ``+++``
sequence compiles to three consecutive ``ADD_IMM v_val, v_val, 1`` instructions.
After constant folding (combined with the peephole merge of consecutive
``ADD_IMM``), this becomes a single ``LOAD_IMM``.

Pattern 2: LOAD_IMM followed by AND_IMM
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

::

    LOAD_IMM  v1, 17    ; v1 = 17 (binary: 00010001)
    AND_IMM   v1, v1, 15; v1 = 17 & 15 = 1 (binary: 00001111 → 00000001)

Becomes:

::

    LOAD_IMM  v1, 1     ; v1 = 1 (folded)

This is common for wrapping values to 8-bit range (``AND_IMM vN, vN, 255``)
or extracting nibbles (``AND_IMM vN, vN, 15``).

The "Pending Load" Approach
----------------------------

Single-pass scan with a ``pending_load`` dict mapping register index to the
known immediate value:

  - On ``LOAD_IMM vN, k``: record ``pending_load[N] = k``
  - On ``ADD_IMM vN, vN, d`` where ``N in pending_load``:
      - Fold: replace both instructions with ``LOAD_IMM vN, pending_load[N]+d``
      - Update ``pending_load[N] = pending_load[N] + d``
  - On ``AND_IMM vN, vN, mask`` where ``N in pending_load``:
      - Fold: replace both instructions with ``LOAD_IMM vN, pending_load[N]&mask``
      - Update ``pending_load[N] = pending_load[N] & mask``
  - On any other instruction that writes to vN: clear ``pending_load[N]``
      (the value is no longer known statically)

When to Clear the Pending Load
--------------------------------

The pending value for vN becomes stale whenever any instruction writes a
non-constant value to vN. We detect "writes to vN" by checking if the first
operand is the register vN and the opcode is not a read-only operation (like
``LOAD_BYTE`` reading into vN, or ``ADD`` where vN is the destination).

For simplicity, we clear the pending load for any instruction where vN appears
as the first operand and the instruction is not a foldable immediate operation.

Example of clearing:

::

    LOAD_IMM  v1, 5   ; pending[1] = 5
    ADD       v1, v2, v3  ; v1 is written with a non-constant → clear pending[1]
    ADD_IMM   v1, v1, 3   ; pending[1] is empty, so we cannot fold → keep as-is

Purity
------

This pass never mutates the input program. It rebuilds the instruction list
from scratch, either copying instructions or replacing two with one.
"""

from __future__ import annotations

from compiler_ir import IrImmediate, IrInstruction, IrProgram, IrRegister
from compiler_ir.opcodes import IrOp

# Opcodes where the destination register is the first operand and the value
# written depends only on an immediate (these are the foldable cases).
_FOLDABLE_IMM_OPS: frozenset[IrOp] = frozenset({IrOp.ADD_IMM, IrOp.AND_IMM})

# Opcodes that write to the first operand (destination register).
# When we see one of these on a register that has a pending load, we must
# check whether it is foldable. If not, clear the pending load.
_WRITES_TO_DEST: frozenset[IrOp] = frozenset(
    {
        IrOp.LOAD_IMM,
        IrOp.LOAD_ADDR,
        IrOp.LOAD_BYTE,
        IrOp.LOAD_WORD,
        IrOp.ADD,
        IrOp.ADD_IMM,
        IrOp.SUB,
        IrOp.AND,
        IrOp.AND_IMM,
        IrOp.CMP_EQ,
        IrOp.CMP_NE,
        IrOp.CMP_LT,
        IrOp.CMP_GT,
    }
)


class ConstantFolder:
    """Fold constant expressions known at compile time.

    Merges ``LOAD_IMM vN, k`` followed immediately by ``ADD_IMM vN, vN, d``
    or ``AND_IMM vN, vN, mask`` into a single ``LOAD_IMM vN, result``.

    The fold is only applied when:

      1. The pending load register matches the destination of the arithmetic op.
      2. The source register of the arithmetic op is also the destination
         (i.e., it is of the form ``OP vN, vN, imm``).
      3. No other instruction modifies vN between the load and the arithmetic.

    Example::

        # Input:  LOAD_IMM v1, 5 ; ADD_IMM v1, v1, 3
        # Output: LOAD_IMM v1, 8
        folder = ConstantFolder()
        result = folder.run(program)
    """

    @property
    def name(self) -> str:
        """The name of this pass, used in diagnostic output.

        Returns:
            ``'ConstantFolder'``
        """
        return "ConstantFolder"

    def run(self, program: IrProgram) -> IrProgram:
        """Fold constant load + arithmetic sequences into single loads.

        Single-pass scan over the instruction list. Uses a ``pending_load``
        dict to track registers with a known compile-time value (from a
        preceding ``LOAD_IMM``). Folds eligible arithmetic operations into
        the load.

        Args:
            program: The IR program to optimize. Not mutated.

        Returns:
            A new ``IrProgram`` with constant expressions folded.
        """
        # pending_load maps register index → known immediate value.
        # A register is in pending_load iff its current value is the constant
        # from a preceding LOAD_IMM that has not yet been overwritten.
        pending_load: dict[int, int] = {}

        # Output instruction list.  We build this fresh; nothing from
        # `program.instructions` is mutated.
        out_instrs: list[IrInstruction] = []

        for instr in program.instructions:
            # ── Case 1: LOAD_IMM vN, k ────────────────────────────────────────
            # Record the pending load so a following ADD_IMM or AND_IMM can
            # fold with it.  We still emit the LOAD_IMM (it may not be folded
            # if no arithmetic follows, or if the arithmetic is on a different
            # register).  If we later fold, we'll pop the pending entry and
            # remove this instruction from the output.
            if instr.opcode == IrOp.LOAD_IMM:
                dest = instr.operands[0]
                imm = instr.operands[1]
                if isinstance(dest, IrRegister) and isinstance(imm, IrImmediate):
                    # Clear any previous pending for this register.
                    pending_load[dest.index] = imm.value
                    out_instrs.append(instr)
                    continue
                # Non-standard LOAD_IMM form — just emit it.
                out_instrs.append(instr)
                continue

            # ── Case 2: ADD_IMM or AND_IMM vN, vN, imm ───────────────────────
            # If vN has a pending load, fold the two instructions into one.
            if instr.opcode in _FOLDABLE_IMM_OPS:
                dest = instr.operands[0] if instr.operands else None
                src = instr.operands[1] if len(instr.operands) > 1 else None
                imm_op = instr.operands[2] if len(instr.operands) > 2 else None

                if (
                    isinstance(dest, IrRegister)
                    and isinstance(src, IrRegister)
                    and isinstance(imm_op, IrImmediate)
                    and dest.index == src.index  # vN, vN form (in-place update)
                    and dest.index in pending_load
                ):
                    # We can fold!  Compute the new value.
                    base_value = pending_load[dest.index]
                    if instr.opcode == IrOp.ADD_IMM:
                        new_value = base_value + imm_op.value
                    else:  # AND_IMM
                        new_value = base_value & imm_op.value

                    # Remove the preceding LOAD_IMM from the output list.
                    # It is always the most recent instruction for this register.
                    # We find it by scanning backwards for LOAD_IMM on dest.index.
                    for i in range(len(out_instrs) - 1, -1, -1):
                        prev = out_instrs[i]
                        if (
                            prev.opcode == IrOp.LOAD_IMM
                            and isinstance(prev.operands[0], IrRegister)
                            and prev.operands[0].index == dest.index
                        ):
                            # Replace the LOAD_IMM with the folded version.
                            out_instrs[i] = IrInstruction(
                                opcode=IrOp.LOAD_IMM,
                                operands=[dest, IrImmediate(new_value)],
                                id=prev.id,  # keep the original instruction ID
                            )
                            break

                    # Update the pending value (the register now holds new_value).
                    pending_load[dest.index] = new_value

                    # Do NOT emit the arithmetic instruction — it has been
                    # absorbed into the preceding LOAD_IMM.
                    continue

            # ── Case 3: Any other instruction ────────────────────────────────
            # If this instruction writes to a register that has a pending load,
            # invalidate the pending load (the value is no longer constant).
            if instr.opcode in _WRITES_TO_DEST and instr.operands:
                dest = instr.operands[0]
                if isinstance(dest, IrRegister) and dest.index in pending_load:
                    del pending_load[dest.index]

            out_instrs.append(instr)

        return IrProgram(
            instructions=out_instrs,
            data=program.data,
            entry_label=program.entry_label,
            version=program.version,
        )
