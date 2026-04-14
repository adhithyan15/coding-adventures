"""Tests for DeadCodeEliminator.

The DeadCodeEliminator removes instructions that follow an unconditional
branch (JUMP, RET, HALT) without an intervening label.

We test:
  - Instructions after JUMP are removed
  - Instructions after RET are removed
  - Instructions after HALT are removed
  - LABEL after a branch is always kept (reachable from elsewhere)
  - Instructions after a conditional branch (BRANCH_Z, BRANCH_NZ) are kept
  - A program with no dead code is returned unchanged (same content)
  - Multiple dead regions are all removed
  - Data declarations and entry_label are preserved
"""

from __future__ import annotations

import pytest

from compiler_ir import IDGenerator, IrImmediate, IrInstruction, IrLabel, IrProgram, IrRegister
from compiler_ir.opcodes import IrOp
from ir_optimizer.passes import DeadCodeEliminator


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_program(*instrs: IrInstruction, entry: str = "_start") -> IrProgram:
    """Build a minimal IrProgram from a list of instructions."""
    prog = IrProgram(entry_label=entry)
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


def label_instr(name: str) -> IrInstruction:
    return IrInstruction(opcode=IrOp.LABEL, operands=[IrLabel(name)], id=-1)


def jump_instr(target: str, id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.JUMP, operands=[IrLabel(target)], id=id_)


def halt_instr(id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.HALT, operands=[], id=id_)


def ret_instr(id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.RET, operands=[], id=id_)


def add_imm_instr(reg: int, val: int, id_: int = 0) -> IrInstruction:
    r = IrRegister(reg)
    return IrInstruction(
        opcode=IrOp.ADD_IMM,
        operands=[r, r, IrImmediate(val)],
        id=id_,
    )


def branch_z_instr(reg: int, target: str, id_: int = 0) -> IrInstruction:
    return IrInstruction(
        opcode=IrOp.BRANCH_Z,
        operands=[IrRegister(reg), IrLabel(target)],
        id=id_,
    )


# ---------------------------------------------------------------------------
# Basic cases
# ---------------------------------------------------------------------------


class TestDeadCodeEliminatorBasic:
    def setup_method(self) -> None:
        self.elim = DeadCodeEliminator()

    def test_name(self) -> None:
        assert self.elim.name == "DeadCodeEliminator"

    def test_empty_program_unchanged(self) -> None:
        prog = make_program()
        result = self.elim.run(prog)
        assert result.instructions == []

    def test_no_dead_code_unchanged(self) -> None:
        """A program with no branches should be returned with all instructions intact."""
        prog = make_program(
            add_imm_instr(1, 1, id_=0),
            add_imm_instr(2, 2, id_=1),
            halt_instr(id_=2),
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 3

    def test_preserves_entry_label(self) -> None:
        prog = make_program(halt_instr(), entry="_start")
        result = self.elim.run(prog)
        assert result.entry_label == "_start"

    def test_preserves_version(self) -> None:
        prog = IrProgram(entry_label="_start", version=2)
        prog.add_instruction(halt_instr())
        result = self.elim.run(prog)
        assert result.version == 2

    def test_does_not_mutate_input(self) -> None:
        prog = make_program(
            jump_instr("end", id_=0),
            add_imm_instr(1, 5, id_=1),  # dead
            label_instr("end"),
            halt_instr(id_=2),
        )
        original_count = len(prog.instructions)
        self.elim.run(prog)
        assert len(prog.instructions) == original_count  # input unchanged


# ---------------------------------------------------------------------------
# Dead code after JUMP
# ---------------------------------------------------------------------------


class TestDeadCodeAfterJump:
    def setup_method(self) -> None:
        self.elim = DeadCodeEliminator()

    def test_removes_code_after_jump(self) -> None:
        """Instructions between JUMP and next LABEL are removed."""
        prog = make_program(
            jump_instr("end", id_=0),
            add_imm_instr(1, 1, id_=1),   # dead
            add_imm_instr(2, 2, id_=2),   # dead
            label_instr("end"),
            halt_instr(id_=3),
        )
        result = self.elim.run(prog)
        opcodes = [i.opcode for i in result.instructions]
        assert IrOp.JUMP in opcodes
        assert IrOp.LABEL in opcodes
        assert IrOp.HALT in opcodes
        # The two ADD_IMM instructions should be gone
        assert opcodes.count(IrOp.ADD_IMM) == 0
        assert len(result.instructions) == 3

    def test_keeps_label_after_jump(self) -> None:
        """A LABEL immediately after a JUMP must be kept."""
        prog = make_program(
            jump_instr("end", id_=0),
            label_instr("end"),
            halt_instr(id_=1),
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 3

    def test_keeps_jump_itself(self) -> None:
        """The JUMP instruction itself is always kept."""
        prog = make_program(
            jump_instr("end", id_=0),
            label_instr("end"),
        )
        result = self.elim.run(prog)
        assert result.instructions[0].opcode == IrOp.JUMP

    def test_multiple_dead_regions_after_jumps(self) -> None:
        """Multiple dead regions are all removed."""
        prog = make_program(
            jump_instr("a", id_=0),
            add_imm_instr(1, 1, id_=1),  # dead
            label_instr("a"),
            jump_instr("b", id_=2),
            add_imm_instr(2, 2, id_=3),  # dead
            label_instr("b"),
            halt_instr(id_=4),
        )
        result = self.elim.run(prog)
        opcodes = [i.opcode for i in result.instructions]
        assert opcodes.count(IrOp.ADD_IMM) == 0
        assert opcodes.count(IrOp.JUMP) == 2
        assert opcodes.count(IrOp.LABEL) == 2
        assert len(result.instructions) == 5


# ---------------------------------------------------------------------------
# Dead code after RET and HALT
# ---------------------------------------------------------------------------


class TestDeadCodeAfterRetAndHalt:
    def setup_method(self) -> None:
        self.elim = DeadCodeEliminator()

    def test_removes_code_after_ret(self) -> None:
        """Instructions after RET are removed."""
        prog = make_program(
            ret_instr(id_=0),
            add_imm_instr(1, 1, id_=1),  # dead
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 1
        assert result.instructions[0].opcode == IrOp.RET

    def test_removes_code_after_halt(self) -> None:
        """Instructions after HALT are removed."""
        prog = make_program(
            halt_instr(id_=0),
            add_imm_instr(1, 1, id_=1),  # dead
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 1
        assert result.instructions[0].opcode == IrOp.HALT

    def test_label_after_halt_is_kept(self) -> None:
        """A label after HALT is always kept (it may be jumped to from elsewhere)."""
        prog = make_program(
            halt_instr(id_=0),
            label_instr("epilogue"),
            add_imm_instr(1, 1, id_=1),  # dead (after label but dead since no jump into epilogue's code reaches here)
        )
        # The label makes subsequent code live again
        result = self.elim.run(prog)
        opcodes = [i.opcode for i in result.instructions]
        assert IrOp.LABEL in opcodes
        # But ADD_IMM after the label IS live
        assert IrOp.ADD_IMM in opcodes

    def test_code_after_label_following_halt_is_live(self) -> None:
        """After HALT → LABEL → instructions, the instructions are live."""
        prog = make_program(
            halt_instr(id_=0),
            label_instr("trap"),
            add_imm_instr(1, 99, id_=1),
            ret_instr(id_=2),
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 4


# ---------------------------------------------------------------------------
# Conditional branches (should NOT eliminate following code)
# ---------------------------------------------------------------------------


class TestConditionalBranches:
    def setup_method(self) -> None:
        self.elim = DeadCodeEliminator()

    def test_branch_z_does_not_eliminate_following_code(self) -> None:
        """BRANCH_Z is conditional — code after it is still reachable via fall-through."""
        prog = make_program(
            branch_z_instr(1, "end", id_=0),
            add_imm_instr(1, 1, id_=1),  # live — fall-through path
            label_instr("end"),
            halt_instr(id_=2),
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 4

    def test_branch_nz_does_not_eliminate_following_code(self) -> None:
        """BRANCH_NZ is conditional — same as BRANCH_Z."""
        prog = make_program(
            IrInstruction(
                opcode=IrOp.BRANCH_NZ,
                operands=[IrRegister(1), IrLabel("end")],
                id=0,
            ),
            add_imm_instr(1, 1, id_=1),  # live
            label_instr("end"),
            halt_instr(id_=2),
        )
        result = self.elim.run(prog)
        assert len(result.instructions) == 4
