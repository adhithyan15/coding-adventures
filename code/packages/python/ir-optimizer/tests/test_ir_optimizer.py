"""Tests for IrOptimizer and OptimizationResult.

Tests the combined optimizer pipeline:
  - default_passes() runs DeadCodeEliminator, ConstantFolder, PeepholeOptimizer
  - no_op() makes no changes
  - OptimizationResult has correct instruction counts
  - Custom pass list works
  - optimize() convenience function works
  - IrPass Protocol is satisfied by the built-in passes
"""

from __future__ import annotations

from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrProgram, IrRegister
from compiler_ir.opcodes import IrOp
from ir_optimizer import IrOptimizer, OptimizationResult, optimize
from ir_optimizer.passes import ConstantFolder, DeadCodeEliminator, PeepholeOptimizer
from ir_optimizer.protocol import IrPass


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


def jump(target: str, id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.JUMP, operands=[IrLabel(target)], id=id_)


def label_instr(name: str) -> IrInstruction:
    return IrInstruction(opcode=IrOp.LABEL, operands=[IrLabel(name)], id=-1)


def halt(id_: int = 0) -> IrInstruction:
    return IrInstruction(opcode=IrOp.HALT, operands=[], id=id_)


# ---------------------------------------------------------------------------
# OptimizationResult
# ---------------------------------------------------------------------------


class TestOptimizationResult:
    def test_instructions_eliminated_positive(self) -> None:
        prog = make_program(halt())
        result = OptimizationResult(
            program=prog,
            passes_run=["TestPass"],
            instructions_before=10,
            instructions_after=7,
        )
        assert result.instructions_eliminated == 3

    def test_instructions_eliminated_zero(self) -> None:
        prog = make_program(halt())
        result = OptimizationResult(
            program=prog,
            passes_run=[],
            instructions_before=5,
            instructions_after=5,
        )
        assert result.instructions_eliminated == 0

    def test_instructions_eliminated_negative(self) -> None:
        """Negative eliminated count is possible (e.g., an instrumentation pass)."""
        prog = make_program(halt())
        result = OptimizationResult(
            program=prog,
            passes_run=[],
            instructions_before=3,
            instructions_after=5,
        )
        assert result.instructions_eliminated == -2

    def test_passes_run_list(self) -> None:
        prog = make_program(halt())
        result = OptimizationResult(
            program=prog,
            passes_run=["A", "B", "C"],
            instructions_before=10,
            instructions_after=8,
        )
        assert result.passes_run == ["A", "B", "C"]


# ---------------------------------------------------------------------------
# IrOptimizer.no_op()
# ---------------------------------------------------------------------------


class TestNoOpOptimizer:
    def test_no_op_returns_same_instructions(self) -> None:
        prog = make_program(
            load_imm(1, 5, id_=0),
            add_imm(1, 1, 3, id_=1),
            halt(id_=2),
        )
        result = IrOptimizer.no_op().optimize(prog)
        assert len(result.program.instructions) == 3

    def test_no_op_instructions_eliminated_is_zero(self) -> None:
        prog = make_program(halt(id_=0), halt(id_=1))
        result = IrOptimizer.no_op().optimize(prog)
        assert result.instructions_eliminated == 0

    def test_no_op_passes_run_is_empty(self) -> None:
        prog = make_program(halt())
        result = IrOptimizer.no_op().optimize(prog)
        assert result.passes_run == []

    def test_no_op_instructions_before_equals_after(self) -> None:
        prog = make_program(halt(id_=0), add_imm(1, 1, 1, id_=1))
        result = IrOptimizer.no_op().optimize(prog)
        assert result.instructions_before == result.instructions_after == 2


# ---------------------------------------------------------------------------
# IrOptimizer.default_passes()
# ---------------------------------------------------------------------------


class TestDefaultPasses:
    def test_default_passes_runs_all_three(self) -> None:
        prog = make_program(halt())
        result = IrOptimizer.default_passes().optimize(prog)
        assert result.passes_run == [
            "DeadCodeEliminator",
            "ConstantFolder",
            "PeepholeOptimizer",
        ]

    def test_default_passes_eliminates_dead_code(self) -> None:
        """The default pipeline should eliminate dead code."""
        prog = make_program(
            jump("end", id_=0),
            add_imm(1, 1, 1, id_=1),  # dead
            label_instr("end"),
            halt(id_=2),
        )
        result = IrOptimizer.default_passes().optimize(prog)
        opcodes = [i.opcode for i in result.program.instructions]
        assert IrOp.ADD_IMM not in opcodes

    def test_default_passes_folds_constants(self) -> None:
        """The default pipeline should fold LOAD_IMM + ADD_IMM."""
        prog = make_program(
            load_imm(1, 5, id_=0),
            add_imm(1, 1, 3, id_=1),
            halt(id_=2),
        )
        result = IrOptimizer.default_passes().optimize(prog)
        instrs = result.program.instructions
        # Should be LOAD_IMM v1, 8 + HALT
        assert len(instrs) == 2
        assert instrs[0].opcode == IrOp.LOAD_IMM
        assert instrs[0].operands[1].value == 8  # type: ignore[union-attr]

    def test_default_passes_merges_add_imm(self) -> None:
        """The default pipeline should merge consecutive ADD_IMM."""
        prog = make_program(
            add_imm(1, 1, 2, id_=0),
            add_imm(1, 1, 3, id_=1),
            halt(id_=2),
        )
        result = IrOptimizer.default_passes().optimize(prog)
        instrs = result.program.instructions
        assert len(instrs) == 2
        add_instrs = [i for i in instrs if i.opcode == IrOp.ADD_IMM]
        assert len(add_instrs) == 1
        assert add_instrs[0].operands[2].value == 5  # type: ignore[union-attr]

    def test_instructions_before_after(self) -> None:
        """instructions_before and instructions_after are correctly set."""
        prog = make_program(
            jump("end", id_=0),
            add_imm(1, 1, 1, id_=1),  # dead — eliminated
            label_instr("end"),
            halt(id_=2),
        )
        result = IrOptimizer.default_passes().optimize(prog)
        assert result.instructions_before == 4
        assert result.instructions_after == 3
        assert result.instructions_eliminated == 1


# ---------------------------------------------------------------------------
# Custom pass list
# ---------------------------------------------------------------------------


class TestCustomPasses:
    def test_single_custom_pass(self) -> None:
        """IrOptimizer with a single pass only runs that pass."""
        prog = make_program(
            jump("end", id_=0),
            add_imm(1, 1, 1, id_=1),  # dead
            label_instr("end"),
            halt(id_=2),
        )
        optimizer = IrOptimizer([DeadCodeEliminator()])
        result = optimizer.optimize(prog)
        assert result.passes_run == ["DeadCodeEliminator"]
        assert IrOp.ADD_IMM not in [i.opcode for i in result.program.instructions]

    def test_two_passes_run_in_order(self) -> None:
        """Passes run in the order given."""
        prog = make_program(halt())
        optimizer = IrOptimizer([ConstantFolder(), DeadCodeEliminator()])
        result = optimizer.optimize(prog)
        assert result.passes_run == ["ConstantFolder", "DeadCodeEliminator"]


# ---------------------------------------------------------------------------
# optimize() convenience function
# ---------------------------------------------------------------------------


class TestOptimizeFunction:
    def test_optimize_default_pipeline(self) -> None:
        """optimize() with no passes argument uses default pipeline."""
        prog = make_program(
            load_imm(1, 5, id_=0),
            add_imm(1, 1, 3, id_=1),
            halt(id_=2),
        )
        result = optimize(prog)
        assert result.passes_run == [
            "DeadCodeEliminator",
            "ConstantFolder",
            "PeepholeOptimizer",
        ]
        assert len(result.program.instructions) == 2

    def test_optimize_with_custom_passes(self) -> None:
        """optimize(program, passes=[...]) uses those passes."""
        prog = make_program(halt())
        result = optimize(prog, passes=[DeadCodeEliminator()])
        assert result.passes_run == ["DeadCodeEliminator"]

    def test_optimize_empty_passes_is_no_op(self) -> None:
        """optimize(program, passes=[]) is equivalent to no_op."""
        prog = make_program(halt(id_=0), halt(id_=1))
        result = optimize(prog, passes=[])
        assert result.instructions_eliminated == 0


# ---------------------------------------------------------------------------
# Protocol satisfaction
# ---------------------------------------------------------------------------


class TestIrPassProtocol:
    def test_dead_code_satisfies_protocol(self) -> None:
        """DeadCodeEliminator satisfies the IrPass Protocol."""
        pass_: IrPass = DeadCodeEliminator()  # type: ignore[assignment]
        assert isinstance(pass_.name, str)

    def test_constant_folder_satisfies_protocol(self) -> None:
        """ConstantFolder satisfies the IrPass Protocol."""
        pass_: IrPass = ConstantFolder()  # type: ignore[assignment]
        assert isinstance(pass_.name, str)

    def test_peephole_satisfies_protocol(self) -> None:
        """PeepholeOptimizer satisfies the IrPass Protocol."""
        pass_: IrPass = PeepholeOptimizer()  # type: ignore[assignment]
        assert isinstance(pass_.name, str)

    def test_custom_pass_accepted_by_optimizer(self) -> None:
        """A custom class satisfying IrPass is accepted by IrOptimizer."""

        class IdentityPass:
            @property
            def name(self) -> str:
                return "IdentityPass"

            def run(self, program: IrProgram) -> IrProgram:
                return program

        prog = make_program(halt())
        optimizer = IrOptimizer([IdentityPass()])
        result = optimizer.optimize(prog)
        assert result.passes_run == ["IdentityPass"]
        assert result.instructions_eliminated == 0
