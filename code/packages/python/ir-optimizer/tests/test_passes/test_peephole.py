"""Tests for PeepholeOptimizer.

The PeepholeOptimizer applies three local instruction-level patterns:

  1. Merge consecutive ADD_IMM on the same register
  2. Remove no-op AND_IMM 255 (when preceded by ADD_IMM/LOAD_IMM with value ≤ 255)
  3. Fold LOAD_IMM 0 + ADD_IMM k into LOAD_IMM k

We test each pattern individually, combined patterns, and the fixed-point
iteration behaviour.
"""

from __future__ import annotations

from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrProgram, IrRegister
from compiler_ir.opcodes import IrOp
from ir_optimizer.passes import PeepholeOptimizer


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_program(*instrs: IrInstruction, entry: str = "_start") -> IrProgram:
    prog = IrProgram(entry_label=entry)
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


def load_imm(reg: int, val: int, id_: int = 0) -> IrInstruction:
    return IrInstruction(
        opcode=IrOp.LOAD_IMM,
        operands=[IrRegister(reg), IrImmediate(val)],
        id=id_,
    )


def add_imm(dst: int, src: int, val: int, id_: int = 0) -> IrInstruction:
    return IrInstruction(
        opcode=IrOp.ADD_IMM,
        operands=[IrRegister(dst), IrRegister(src), IrImmediate(val)],
        id=id_,
    )


def and_imm(dst: int, src: int, mask: int, id_: int = 0) -> IrInstruction:
    return IrInstruction(
        opcode=IrOp.AND_IMM,
        operands=[IrRegister(dst), IrRegister(src), IrImmediate(mask)],
        id=id_,
    )


def halt(id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.HALT, operands=[], id=id_)


def label_instr(name: str) -> IrInstruction:
    return IrInstruction(opcode=IrOp.LABEL, operands=[IrLabel(name)], id=-1)


# ---------------------------------------------------------------------------
# Basic
# ---------------------------------------------------------------------------


class TestPeepholeBasic:
    def setup_method(self) -> None:
        self.opt = PeepholeOptimizer()

    def test_name(self) -> None:
        assert self.opt.name == "PeepholeOptimizer"

    def test_empty_program_unchanged(self) -> None:
        prog = make_program()
        result = self.opt.run(prog)
        assert result.instructions == []

    def test_single_instruction_unchanged(self) -> None:
        prog = make_program(halt(id_=0))
        result = self.opt.run(prog)
        assert len(result.instructions) == 1

    def test_preserves_entry_label(self) -> None:
        prog = make_program(halt(), entry="main")
        result = self.opt.run(prog)
        assert result.entry_label == "main"

    def test_does_not_mutate_input(self) -> None:
        prog = make_program(
            add_imm(1, 1, 3, id_=0),
            add_imm(1, 1, 2, id_=1),
        )
        original_count = len(prog.instructions)
        self.opt.run(prog)
        assert len(prog.instructions) == original_count


# ---------------------------------------------------------------------------
# Pattern 1: Merge consecutive ADD_IMM on the same register
# ---------------------------------------------------------------------------


class TestMergeConsecutiveAddImm:
    def setup_method(self) -> None:
        self.opt = PeepholeOptimizer()

    def test_merges_two_add_imm(self) -> None:
        """ADD_IMM v1, v1, 3 ; ADD_IMM v1, v1, 2 → ADD_IMM v1, v1, 5"""
        prog = make_program(
            add_imm(1, 1, 3, id_=0),
            add_imm(1, 1, 2, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 1
        instr = result.instructions[0]
        assert instr.opcode == IrOp.ADD_IMM
        assert instr.operands[2].value == 5  # type: ignore[union-attr]

    def test_merge_preserves_first_id(self) -> None:
        """Merged instruction keeps the first ADD_IMM's ID."""
        prog = make_program(
            add_imm(1, 1, 3, id_=10),
            add_imm(1, 1, 2, id_=11),
        )
        result = self.opt.run(prog)
        assert result.instructions[0].id == 10

    def test_no_merge_different_registers(self) -> None:
        """ADD_IMM v1 and ADD_IMM v2 are not merged."""
        prog = make_program(
            add_imm(1, 1, 3, id_=0),
            add_imm(2, 2, 2, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 2

    def test_no_merge_non_inplace(self) -> None:
        """ADD_IMM v2, v1, 3 is not an in-place update — no merge."""
        prog = make_program(
            add_imm(1, 1, 3, id_=0),
            IrInstruction(
                opcode=IrOp.ADD_IMM,
                operands=[IrRegister(2), IrRegister(1), IrImmediate(2)],
                id=1,
            ),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 2

    def test_merges_three_consecutive_via_iteration(self) -> None:
        """Three consecutive ADD_IMM merge to one via fixed-point iteration."""
        prog = make_program(
            add_imm(1, 1, 1, id_=0),
            add_imm(1, 1, 1, id_=1),
            add_imm(1, 1, 1, id_=2),
        )
        result = self.opt.run(prog)
        # First iteration: [0,1] merge to 2, [2] is alone → [2-ADD_IMM, orig-2]
        # Second iteration: those two merge to 3
        assert len(result.instructions) == 1
        assert result.instructions[0].operands[2].value == 3  # type: ignore[union-attr]

    def test_merge_with_negative_immediate(self) -> None:
        """ADD_IMM v1, v1, 5 ; ADD_IMM v1, v1, -3 → ADD_IMM v1, v1, 2"""
        prog = make_program(
            add_imm(1, 1, 5, id_=0),
            add_imm(1, 1, -3, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 1
        assert result.instructions[0].operands[2].value == 2  # type: ignore[union-attr]


# ---------------------------------------------------------------------------
# Pattern 2: Remove no-op AND_IMM 255
# ---------------------------------------------------------------------------


class TestRemoveAndImm255:
    def setup_method(self) -> None:
        self.opt = PeepholeOptimizer()

    def test_removes_and_imm_255_after_add_imm(self) -> None:
        """ADD_IMM v1, v1, 1 ; AND_IMM v1, v1, 255 → ADD_IMM v1, v1, 1"""
        prog = make_program(
            add_imm(1, 1, 1, id_=0),
            and_imm(1, 1, 255, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 1
        assert result.instructions[0].opcode == IrOp.ADD_IMM

    def test_removes_and_imm_255_after_load_imm(self) -> None:
        """LOAD_IMM v1, 100 ; AND_IMM v1, v1, 255 → LOAD_IMM v1, 100"""
        prog = make_program(
            load_imm(1, 100, id_=0),
            and_imm(1, 1, 255, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 1
        assert result.instructions[0].opcode == IrOp.LOAD_IMM

    def test_keeps_and_imm_255_when_value_exceeds_255(self) -> None:
        """AND_IMM 255 after LOAD_IMM with value > 255 is NOT removed (it's meaningful)."""
        prog = make_program(
            load_imm(1, 300, id_=0),   # > 255 — the AND does something
            and_imm(1, 1, 255, id_=1),
        )
        result = self.opt.run(prog)
        # This case should NOT be removed because 300 > 255
        assert len(result.instructions) == 2

    def test_keeps_and_imm_with_mask_not_255(self) -> None:
        """AND_IMM v1, v1, 15 is NOT removed (mask != 255)."""
        prog = make_program(
            add_imm(1, 1, 5, id_=0),
            and_imm(1, 1, 15, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 2

    def test_keeps_and_imm_255_on_different_register(self) -> None:
        """AND_IMM v2, v2, 255 after ADD_IMM v1 is not removed (different register)."""
        prog = make_program(
            add_imm(1, 1, 5, id_=0),
            and_imm(2, 2, 255, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 2


# ---------------------------------------------------------------------------
# Pattern 3: LOAD_IMM 0 + ADD_IMM k → LOAD_IMM k
# ---------------------------------------------------------------------------


class TestFoldLoadZeroAddImm:
    def setup_method(self) -> None:
        self.opt = PeepholeOptimizer()

    def test_folds_load_zero_then_add(self) -> None:
        """LOAD_IMM v1, 0 ; ADD_IMM v1, v1, 7 → LOAD_IMM v1, 7"""
        prog = make_program(
            load_imm(1, 0, id_=0),
            add_imm(1, 1, 7, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 1
        instr = result.instructions[0]
        assert instr.opcode == IrOp.LOAD_IMM
        assert instr.operands[1].value == 7  # type: ignore[union-attr]

    def test_folds_load_zero_add_preserves_id(self) -> None:
        """Folded LOAD_IMM k keeps the original LOAD_IMM's ID."""
        prog = make_program(
            load_imm(1, 0, id_=99),
            add_imm(1, 1, 42, id_=100),
        )
        result = self.opt.run(prog)
        assert result.instructions[0].id == 99

    def test_does_not_fold_load_nonzero_add(self) -> None:
        """LOAD_IMM v1, 5 ; ADD_IMM v1, v1, 3 is NOT pattern 3 (handled by ConstantFolder)."""
        # Peephole Pattern 3 only fires on LOAD_IMM 0.
        prog = make_program(
            load_imm(1, 5, id_=0),
            add_imm(1, 1, 3, id_=1),
        )
        result = self.opt.run(prog)
        # NOT folded by peephole (that's ConstantFolder's job)
        assert len(result.instructions) == 2

    def test_does_not_fold_load_zero_different_register(self) -> None:
        """LOAD_IMM v1, 0 ; ADD_IMM v2, v2, 3 does not fold (different registers)."""
        prog = make_program(
            load_imm(1, 0, id_=0),
            add_imm(2, 2, 3, id_=1),
        )
        result = self.opt.run(prog)
        assert len(result.instructions) == 2


# ---------------------------------------------------------------------------
# Interaction between patterns and iteration
# ---------------------------------------------------------------------------


class TestPeepholeInteractions:
    def setup_method(self) -> None:
        self.opt = PeepholeOptimizer()

    def test_pattern_1_and_3_interaction(self) -> None:
        """LOAD_IMM 0, ADD_IMM 1, ADD_IMM 2 → eventually LOAD_IMM 3 via iteration.

        Iteration 1: Pattern 3 fires on (LOAD_IMM 0, ADD_IMM 1) → LOAD_IMM 1
                     Remaining: [LOAD_IMM 1, ADD_IMM 2]
        Iteration 2: No pattern fires (Pattern 3 requires LOAD_IMM 0, not LOAD_IMM 1;
                     Pattern 1 requires ADD_IMM + ADD_IMM).
                     Fixed point with 2 instructions.
        """
        prog = make_program(
            load_imm(1, 0, id_=0),
            add_imm(1, 1, 1, id_=1),
            add_imm(1, 1, 2, id_=2),
        )
        result = self.opt.run(prog)
        # After P3: [LOAD_IMM v1, 1, ADD_IMM v1, v1, 2]
        # P3 doesn't fire again (not zero), P1 doesn't fire (LOAD_IMM + ADD_IMM)
        # So we expect 2 instructions
        assert len(result.instructions) == 2

    def test_multiple_patterns_same_program(self) -> None:
        """A program with all three pattern types."""
        prog = make_program(
            load_imm(1, 0, id_=0),         # P3 with next
            add_imm(1, 1, 5, id_=1),       # → LOAD_IMM v1, 5
            add_imm(2, 2, 3, id_=2),       # P1 with next
            add_imm(2, 2, 4, id_=3),       # → ADD_IMM v2, v2, 7
            add_imm(3, 3, 10, id_=4),      # P2: ADD_IMM ≤ 255
            and_imm(3, 3, 255, id_=5),     # → removed
            halt(id_=6),
        )
        result = self.opt.run(prog)
        # After one pass:
        #   P3 fires on (0,1) → LOAD_IMM v1, 5; skip to index 2
        #   P1 fires on (2,3) → ADD_IMM v2, v2, 7; skip to index 4
        #   P2 fires on (4,5) → ADD_IMM v3, v3, 10; skip to index 6
        #   HALT unchanged
        # Result: 4 instructions
        opcodes = [i.opcode for i in result.instructions]
        assert IrOp.HALT in opcodes
        assert len(result.instructions) == 4
