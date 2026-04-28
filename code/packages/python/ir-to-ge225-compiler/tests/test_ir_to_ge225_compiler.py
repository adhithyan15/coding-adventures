"""Tests for the ir_to_ge225_compiler package.

Each test compiles an IrProgram, loads the binary into a GE225Simulator, runs
it to completion, and then verifies the simulator state (memory contents or
typewriter output).  This approach validates both the code generator and its
interaction with the real GE-225 simulator.

Test coverage plan (from spec IR01):
- HALT: PC reaches halt_address; simulator stops there.
- LOAD_IMM + ADD: arithmetic in memory.
- SUB, MUL, DIV.
- CMP_EQ / CMP_NE / CMP_LT / CMP_GT: true and false cases.
- JUMP: unconditional skip.
- BRANCH_Z / BRANCH_NZ: conditional skip.
- FOR-like countdown loop.
- SYSCALL 1: typewriter output.
- AND_IMM 1: parity extraction (0 and 1 inputs).
- AND_IMM non-1: CodeGenError.
- ADD_IMM imm=0 (copy), +1, -1, other.
- Negative constants.
- Undefined label: CodeGenError.
- Unsupported opcode: CodeGenError.
- CompileResult fields: binary, halt_address, data_base, label_map.
"""

from __future__ import annotations

import pytest
from compiler_ir import (
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from ge225_simulator import GE225Simulator

from ir_to_ge225_compiler import (
    CodeGenError,
    CompileResult,
    compile_to_ge225,
    validate_for_ge225,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

_MASK20 = (1 << 20) - 1
_SIGN_BIT = 1 << 19


def _instr(op: IrOp, *operands: IrImmediate | IrRegister | IrLabel) -> IrInstruction:
    """Convenience constructor for IrInstruction."""
    return IrInstruction(opcode=op, operands=list(operands), id=-1)


def _reg(n: int) -> IrRegister:
    return IrRegister(index=n)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _lbl(name: str) -> IrLabel:
    return IrLabel(name=name)


def _prog(*instrs: IrInstruction) -> IrProgram:
    """Build an IrProgram from a flat instruction list."""
    prog = IrProgram(entry_label="_start")
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


def _compile(program: IrProgram) -> tuple[GE225Simulator, CompileResult]:
    """Compile and run a program to completion.

    Returns ``(sim, result)`` where ``sim`` is the post-execution simulator and
    ``result`` is the CompileResult.

    Runs at most 10 000 steps to guard against accidental infinite loops in
    buggy test programs.
    """
    result = compile_to_ge225(program)
    sim = GE225Simulator(memory_words=8192)
    sim.load_program_bytes(result.binary)
    for _ in range(10_000):
        trace = sim.step()
        if trace.address == result.halt_address:
            break
    return sim, result


def _read_signed(sim: GE225Simulator, addr: int) -> int:
    """Read a GE-225 memory word and sign-extend it to a Python int."""
    raw = sim.read_word(addr)
    return raw - (1 << 20) if raw & _SIGN_BIT else raw


def _spill(result: CompileResult, reg_index: int) -> int:
    """Return the absolute address of spill slot for virtual register vN."""
    return result.data_base + reg_index


# ---------------------------------------------------------------------------
# HALT
# ---------------------------------------------------------------------------


class TestHalt:
    """Programs that end with HALT."""

    def test_halt_only_reaches_halt_address(self) -> None:
        """A program with only HALT should execute the halt stub."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.HALT),
        )
        result = compile_to_ge225(program)
        sim = GE225Simulator(memory_words=4096)
        sim.load_program_bytes(result.binary)
        trace = sim.step()  # TON (prologue)
        trace = sim.step()  # BRU halt_address (the HALT IR instruction)
        trace = sim.step()  # BRU halt_address (the halt stub itself)
        assert trace.address == result.halt_address

    def test_compile_result_fields(self) -> None:
        """CompileResult carries binary, halt_address, data_base, label_map."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.HALT),
        )
        result = compile_to_ge225(program)
        assert isinstance(result.binary, bytes)
        assert result.halt_address > 0
        assert result.data_base == result.halt_address + 1
        assert "_start" in result.label_map
        assert result.label_map["_start"] == 1  # TON is at addr 0, _start at addr 1


# ---------------------------------------------------------------------------
# LOAD_IMM
# ---------------------------------------------------------------------------


class TestLoadImm:
    """LOAD_IMM stores a constant in a virtual register's spill slot."""

    def test_load_imm_positive(self) -> None:
        """LOAD_IMM v1, 42 → spill(v1) == 42."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(42)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 42

    def test_load_imm_zero(self) -> None:
        """LOAD_IMM v1, 0 → spill(v1) == 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(0)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 0

    def test_load_imm_negative(self) -> None:
        """LOAD_IMM v1, -7 → spill(v1) holds -7 in two's complement."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(-7)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert _read_signed(sim, _spill(result, 1)) == -7

    def test_same_constant_used_twice_shares_table_entry(self) -> None:
        """Two LOAD_IMM with the same value use one constants-table entry."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(99)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(99)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 99
        assert sim.read_word(_spill(result, 2)) == 99


# ---------------------------------------------------------------------------
# ADD / SUB
# ---------------------------------------------------------------------------


class TestAddSub:
    """Register-register addition and subtraction."""

    def test_add(self) -> None:
        """v3 = v1 + v2 → 3 + 4 = 7."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(3)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(4)),
            _instr(IrOp.ADD, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 7

    def test_sub(self) -> None:
        """v3 = v1 - v2 → 10 - 3 = 7."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(10)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(3)),
            _instr(IrOp.SUB, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 7

    def test_add_negative_result(self) -> None:
        """v3 = 2 - 5 = -3."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(2)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.SUB, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert _read_signed(sim, _spill(result, 3)) == -3


# ---------------------------------------------------------------------------
# ADD_IMM
# ---------------------------------------------------------------------------


class TestAddImm:
    """ADD_IMM specialisations: copy (0), ±1, and general constant."""

    def test_add_imm_copy(self) -> None:
        """ADD_IMM v2, v1, 0 is a register copy."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(55)),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(0)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 55

    def test_add_imm_plus_one(self) -> None:
        """ADD_IMM v2, v1, 1 increments by one."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(10)),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 11

    def test_add_imm_minus_one(self) -> None:
        """ADD_IMM v2, v1, -1 decrements by one."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(7)),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(-1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 6

    def test_add_imm_general(self) -> None:
        """ADD_IMM v2, v1, 100 adds via constants table."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(100)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 105

    def test_add_imm_negative_general(self) -> None:
        """ADD_IMM v2, v1, -10 adds a negative constant (two's complement)."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(15)),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(-10)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 5


# ---------------------------------------------------------------------------
# MUL / DIV
# ---------------------------------------------------------------------------


class TestMulDiv:
    """Multiply and integer divide."""

    def test_mul(self) -> None:
        """v3 = v1 * v2 → 6 × 7 = 42."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(6)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(7)),
            _instr(IrOp.MUL, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 42

    def test_div_quotient(self) -> None:
        """v3 = v1 / v2 → 15 ÷ 4 = 3 (truncates)."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(15)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(4)),
            _instr(IrOp.DIV, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 3

    def test_div_exact(self) -> None:
        """v3 = 12 / 3 = 4 (no remainder)."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(12)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(3)),
            _instr(IrOp.DIV, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 4

    def test_mul_by_zero(self) -> None:
        """v3 = v1 * 0 = 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(99)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(0)),
            _instr(IrOp.MUL, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0


# ---------------------------------------------------------------------------
# CMP_EQ / CMP_NE
# ---------------------------------------------------------------------------


class TestCmpEqNe:
    """Equality comparisons produce 0 (false) or 1 (true)."""

    def test_cmp_eq_true(self) -> None:
        """Equal inputs → result 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_EQ, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 1

    def test_cmp_eq_false(self) -> None:
        """Unequal inputs → result 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(3)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(7)),
            _instr(IrOp.CMP_EQ, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0

    def test_cmp_ne_true(self) -> None:
        """Unequal inputs → CMP_NE result 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(3)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(7)),
            _instr(IrOp.CMP_NE, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 1

    def test_cmp_ne_false(self) -> None:
        """Equal inputs → CMP_NE result 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_NE, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0


# ---------------------------------------------------------------------------
# CMP_LT / CMP_GT
# ---------------------------------------------------------------------------


class TestCmpLtGt:
    """Signed less-than and greater-than comparisons."""

    def test_cmp_lt_true(self) -> None:
        """2 < 5 → 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(2)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_LT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 1

    def test_cmp_lt_false_equal(self) -> None:
        """5 < 5 → 0 (equal is not less than)."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_LT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0

    def test_cmp_lt_false_greater(self) -> None:
        """7 < 3 → 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(7)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(3)),
            _instr(IrOp.CMP_LT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0

    def test_cmp_gt_true(self) -> None:
        """5 > 2 → 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(2)),
            _instr(IrOp.CMP_GT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 1

    def test_cmp_gt_false(self) -> None:
        """2 > 5 → 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(2)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_GT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0

    def test_cmp_gt_false_equal(self) -> None:
        """5 > 5 → 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(5)),
            _instr(IrOp.CMP_GT, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 3)) == 0


# ---------------------------------------------------------------------------
# JUMP
# ---------------------------------------------------------------------------


class TestJump:
    """Unconditional branch."""

    def test_jump_skips_instruction(self) -> None:
        """JUMP over a LOAD_IMM: the skipped instruction's register stays 0."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(10)),
            _instr(IrOp.JUMP, _lbl("_after")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(99)),  # should be skipped
            _instr(IrOp.LABEL, _lbl("_after")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 10  # 99 was never written


# ---------------------------------------------------------------------------
# BRANCH_Z / BRANCH_NZ
# ---------------------------------------------------------------------------


class TestBranch:
    """Conditional branches."""

    def test_branch_z_taken_when_zero(self) -> None:
        """BRANCH_Z with zero register jumps over the skipped LOAD_IMM."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(0)),   # v1 = 0 → condition zero
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(42)),   # v2 = 42 (sentinel)
            _instr(IrOp.BRANCH_Z, _reg(1), _lbl("_skip")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(99)),   # skipped when zero
            _instr(IrOp.LABEL, _lbl("_skip")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 42

    def test_branch_z_not_taken_when_nonzero(self) -> None:
        """BRANCH_Z with non-zero register falls through."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),    # v1 = 5 → not zero
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(0)),
            _instr(IrOp.BRANCH_Z, _reg(1), _lbl("_skip")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(77)),   # executed (not skipped)
            _instr(IrOp.LABEL, _lbl("_skip")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 77

    def test_branch_nz_taken_when_nonzero(self) -> None:
        """BRANCH_NZ with non-zero register jumps."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(3)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(55)),
            _instr(IrOp.BRANCH_NZ, _reg(1), _lbl("_skip")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(99)),   # skipped
            _instr(IrOp.LABEL, _lbl("_skip")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 55

    def test_branch_nz_not_taken_when_zero(self) -> None:
        """BRANCH_NZ with zero register falls through."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(0)),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(0)),
            _instr(IrOp.BRANCH_NZ, _reg(1), _lbl("_skip")),
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(33)),   # executed
            _instr(IrOp.LABEL, _lbl("_skip")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 33


# ---------------------------------------------------------------------------
# Loop (FOR-like countdown)
# ---------------------------------------------------------------------------


class TestLoop:
    """A countdown loop demonstrates LABEL, CMP_GT, BRANCH_NZ, ADD_IMM, JUMP."""

    def test_countdown_loop(self) -> None:
        """Count from 3 down to 0, accumulating into v2.

        v1 = 3 (counter), v2 = 0 (accumulator), v3 = 0 (limit)
        while v1 > v3:
            v2 += 1
            v1 -= 1

        After the loop, v1 == 0, v2 == 3.
        """
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(3)),   # counter = 3
            _instr(IrOp.LOAD_IMM, _reg(2), _imm(0)),   # accumulator = 0
            _instr(IrOp.LOAD_IMM, _reg(3), _imm(0)),   # limit = 0
            _instr(IrOp.LABEL, _lbl("_check")),
            _instr(IrOp.CMP_GT, _reg(4), _reg(1), _reg(3)),    # v4 = (v1 > v3)
            _instr(IrOp.BRANCH_NZ, _reg(4), _lbl("_body")),    # jump if v1 > 0
            _instr(IrOp.JUMP, _lbl("_end")),
            _instr(IrOp.LABEL, _lbl("_body")),
            _instr(IrOp.ADD_IMM, _reg(2), _reg(2), _imm(1)),   # acc += 1
            _instr(IrOp.ADD_IMM, _reg(1), _reg(1), _imm(-1)),  # counter -= 1
            _instr(IrOp.JUMP, _lbl("_check")),
            _instr(IrOp.LABEL, _lbl("_end")),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 0  # counter exhausted
        assert sim.read_word(_spill(result, 2)) == 3  # accumulator = 3


# ---------------------------------------------------------------------------
# AND_IMM
# ---------------------------------------------------------------------------


class TestAndImm:
    """AND_IMM v, v, 1 — parity bit extraction."""

    def test_and_imm_1_even_input(self) -> None:
        """AND_IMM of an even value produces 0."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(6)),   # 6 is even
            _instr(IrOp.AND_IMM, _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 0

    def test_and_imm_1_odd_input(self) -> None:
        """AND_IMM of an odd value produces 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(7)),   # 7 is odd
            _instr(IrOp.AND_IMM, _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 1

    def test_and_imm_1_value_1(self) -> None:
        """AND_IMM with input 1 (odd) → result 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(1)),
            _instr(IrOp.AND_IMM, _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 2)) == 1

    def test_and_imm_non_1_raises(self) -> None:
        """AND_IMM with immediate != 1 raises CodeGenError."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(0xFF)),
            _instr(IrOp.AND_IMM, _reg(2), _reg(1), _imm(0xFF)),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="AND_IMM"):
            compile_to_ge225(program)


# ---------------------------------------------------------------------------
# SYSCALL 1 (typewriter output)
# ---------------------------------------------------------------------------


class TestSyscall:
    """SYSCALL 1 prints the GE-225 typewriter character whose code is in v0."""

    def test_syscall_prints_character(self) -> None:
        """Print 'A' (GE-225 typewriter code 0o21 = 17)."""
        a_code = 0o21  # octal 21 = 17 decimal = 'A' in GE-225 typewriter codes
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(0), _imm(a_code)),
            _instr(IrOp.SYSCALL, _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.get_typewriter_output() == "A"

    def test_syscall_prints_multiple_characters(self) -> None:
        """Print 'HI' — two sequential SYSCALL 1 invocations."""
        # H = 0o30 = 24; I = 0o31 = 25
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(0), _imm(0o30)),
            _instr(IrOp.SYSCALL, _imm(1)),
            _instr(IrOp.LOAD_IMM, _reg(0), _imm(0o31)),
            _instr(IrOp.SYSCALL, _imm(1)),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.get_typewriter_output() == "HI"

    def test_syscall_non_1_raises(self) -> None:
        """SYSCALL with number != 1 raises CodeGenError."""
        program = _prog(
            _instr(IrOp.SYSCALL, _imm(2)),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="SYSCALL"):
            compile_to_ge225(program)


# ---------------------------------------------------------------------------
# NOP
# ---------------------------------------------------------------------------


class TestNop:
    """NOP is a no-operation."""

    def test_nop_does_nothing(self) -> None:
        """NOP leaves a register's spill slot unchanged."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(7)),
            _instr(IrOp.NOP),
            _instr(IrOp.HALT),
        )
        sim, result = _compile(program)
        assert sim.read_word(_spill(result, 1)) == 7


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestErrors:
    """CodeGenError for unsupported features."""

    def test_undefined_label_raises(self) -> None:
        """A branch to an undefined label raises CodeGenError."""
        program = _prog(
            _instr(IrOp.JUMP, _lbl("_nowhere")),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="undefined label"):
            compile_to_ge225(program)

    def test_unsupported_opcode_load_byte_raises(self) -> None:
        """LOAD_BYTE is not supported in V1."""
        program = _prog(
            _instr(IrOp.LOAD_BYTE, _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="unsupported"):
            compile_to_ge225(program)

    def test_unsupported_opcode_call_raises(self) -> None:
        """CALL is not supported in V1."""
        program = _prog(
            _instr(IrOp.CALL, _lbl("_func")),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="unsupported"):
            compile_to_ge225(program)

    def test_unsupported_opcode_and_raises(self) -> None:
        """AND (register-register) is not supported in V1."""
        program = _prog(
            _instr(IrOp.AND, _reg(3), _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        with pytest.raises(CodeGenError, match="unsupported"):
            compile_to_ge225(program)

    def test_label_map_contains_defined_labels(self) -> None:
        """CompileResult.label_map contains all LABEL instructions."""
        program = _prog(
            _instr(IrOp.LABEL, _lbl("_start")),
            _instr(IrOp.LABEL, _lbl("_loop")),
            _instr(IrOp.HALT),
        )
        result = compile_to_ge225(program)
        assert "_start" in result.label_map
        assert "_loop" in result.label_map

    def test_data_base_is_code_end_plus_one(self) -> None:
        """data_base == halt_address + 1."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(1)),
            _instr(IrOp.HALT),
        )
        result = compile_to_ge225(program)
        assert result.data_base == result.halt_address + 1

    def test_binary_length_correct(self) -> None:
        """Binary length = (code_end + 1 + n_regs + n_consts) × 3 bytes."""
        # v1 and one constant (5)
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(5)),
            _instr(IrOp.HALT),
        )
        result = compile_to_ge225(program)
        # code: TON(1) + LOAD_IMM(2) + HALT(1) = 4 words; halt_stub = 1 word
        # data: n_regs=2 (v0, v1) + n_consts=1 (5) = 3 words; total = 8 words = 24 bytes
        assert len(result.binary) % 3 == 0  # always a multiple of 3
        n_words = len(result.binary) // 3
        # data_base + n_regs + n_consts = total size
        expected = result.data_base + 2 + 1  # 2 spill slots (v0,v1), 1 const
        assert n_words == expected


# ---------------------------------------------------------------------------
# Pre-flight validator tests
# ---------------------------------------------------------------------------


class TestValidateForGe225:
    """Tests for validate_for_ge225() — the pre-flight IR inspector.

    Each test checks a specific constraint rule in isolation.  The validator
    must return an empty list for valid IR and a non-empty list (with a
    meaningful diagnostic) for any violation.
    """

    def test_valid_program_returns_no_errors(self) -> None:
        """A well-formed program with only supported opcodes and in-range
        constants should produce zero validation errors."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(42)),
            _instr(IrOp.ADD_IMM,  _reg(2), _reg(1), _imm(1)),
            _instr(IrOp.SYSCALL,  _imm(1), _reg(0)),
            _instr(IrOp.HALT),
        )
        assert validate_for_ge225(program) == []

    # ── Rule 1: unsupported opcodes ──────────────────────────────────────────

    def test_load_byte_opcode_rejected(self) -> None:
        """LOAD_BYTE is not in the GE-225 V1 backend's opcode set."""
        program = _prog(_instr(IrOp.LOAD_BYTE, _reg(0), _reg(1), _imm(0)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "unsupported" in errors[0]
        assert "LOAD_BYTE" in errors[0]

    def test_call_opcode_rejected(self) -> None:
        """CALL is not in the GE-225 V1 backend's opcode set."""
        program = _prog(_instr(IrOp.CALL, IrLabel("f")))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "unsupported" in errors[0]
        assert "CALL" in errors[0]

    def test_and_opcode_rejected(self) -> None:
        """Plain AND (register-register) is not supported; only AND_IMM 1 is."""
        program = _prog(_instr(IrOp.AND, _reg(0), _reg(1), _reg(2)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "unsupported" in errors[0]
        assert "AND" in errors[0]

    def test_bitwise_opcodes_rejected(self) -> None:
        """OR, OR_IMM, XOR, XOR_IMM, and NOT were added in compiler-ir v0.3.0
        for the Oct/Intel-8008 target.  The GE-225 V1 backend does not support
        them (the GE-225 has no bitwise OR/XOR instructions).  Each must produce
        an 'unsupported opcode' diagnostic."""
        unsupported = (IrOp.OR, IrOp.OR_IMM, IrOp.XOR, IrOp.XOR_IMM, IrOp.NOT)
        for op in unsupported:
            program = _prog(_instr(op))
            errors = validate_for_ge225(program)
            assert any("unsupported" in e for e in errors), (
                f"IrOp.{op.name} should be rejected by the GE-225 opcode-support "
                f"check but was accepted"
            )
            assert any(op.name in e for e in errors), (
                f"Error for IrOp.{op.name} does not mention the opcode name: "
                f"{errors!r}"
            )

    def test_multiple_unsupported_opcodes_all_reported(self) -> None:
        """Every unsupported opcode in the program generates its own error."""
        program = _prog(
            _instr(IrOp.LOAD_BYTE,  _reg(0), _reg(1), _imm(0)),
            _instr(IrOp.STORE_BYTE, _reg(0), _reg(1), _imm(0)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert len(errors) == 2

    # ── Rule 2: constant overflow ────────────────────────────────────────────

    def test_load_imm_constant_too_large_rejected(self) -> None:
        """1 000 000 000 overflows a 20-bit signed word (max 524 287)."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(1_000_000_000)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "1,000,000,000" in errors[0]

    def test_load_imm_max_in_range_accepted(self) -> None:
        """524 287 (= 2^19 − 1) is the largest valid GE-225 positive constant."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(524_287)),
            _instr(IrOp.HALT),
        )
        assert validate_for_ge225(program) == []

    def test_load_imm_min_in_range_accepted(self) -> None:
        """-524 288 (= −2^19) is the smallest valid GE-225 constant."""
        program = _prog(
            _instr(IrOp.LOAD_IMM, _reg(1), _imm(-524_288)),
            _instr(IrOp.HALT),
        )
        assert validate_for_ge225(program) == []

    def test_load_imm_one_above_max_rejected(self) -> None:
        """524 288 is exactly one above the 20-bit maximum."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(524_288)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "524,288" in errors[0]

    def test_add_imm_overflow_rejected(self) -> None:
        """ADD_IMM with an oversized constant is also caught."""
        program = _prog(
            _instr(IrOp.ADD_IMM, _reg(2), _reg(1), _imm(600_000)),
        )
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "600,000" in errors[0]

    # ── Rule 3: unsupported SYSCALL numbers ─────────────────────────────────

    def test_syscall_1_accepted(self) -> None:
        """SYSCALL 1 (print char) is the only supported syscall."""
        program = _prog(
            _instr(IrOp.SYSCALL, _imm(1), _reg(0)),
            _instr(IrOp.HALT),
        )
        assert validate_for_ge225(program) == []

    def test_syscall_2_rejected(self) -> None:
        """SYSCALL 2 is not wired up in the V1 GE-225 backend."""
        program = _prog(_instr(IrOp.SYSCALL, _imm(2), _reg(0)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "unsupported SYSCALL" in errors[0]
        assert "2" in errors[0]

    # ── Rule 4: AND_IMM immediate ────────────────────────────────────────────

    def test_and_imm_2_rejected(self) -> None:
        """AND_IMM with immediate != 1 is unsupported."""
        program = _prog(_instr(IrOp.AND_IMM, _reg(0), _reg(1), _imm(2)))
        errors = validate_for_ge225(program)
        assert len(errors) == 1
        assert "unsupported" in errors[0]

    # ── Integration: compile_to_ge225 calls validate first ──────────────────

    def test_compile_to_ge225_rejects_oversized_constant(self) -> None:
        """compile_to_ge225 must raise CodeGenError (not silently corrupt data)
        when the IR contains a constant that overflows a 20-bit word."""
        program = _prog(_instr(IrOp.LOAD_IMM, _reg(1), _imm(1_000_000_000)))
        with pytest.raises(CodeGenError, match="pre-flight"):
            compile_to_ge225(program)

    def test_compile_to_ge225_rejects_unsupported_opcode_before_codegen(self) -> None:
        """The error must say 'pre-flight' (raised by the validator, not mid-
        codegen) so callers know the binary is still clean."""
        program = _prog(_instr(IrOp.LOAD_BYTE, _reg(0), _reg(1), _imm(0)))
        with pytest.raises(CodeGenError, match="pre-flight"):
            compile_to_ge225(program)


# ---------------------------------------------------------------------------
# Oct IR compatibility — explicit rejection tests
# ---------------------------------------------------------------------------
#
# Oct compiles to a general-purpose IR that targets the Intel 8008 and
# cross-platform backends (WASM, JVM, CLR).  The GE-225 V1 backend cannot
# execute Oct IR because:
#
#   1. Oct always emits CALL / RET (function calls are in every Oct program;
#      the entry stub is CALL _fn_main … HALT).
#   2. Oct emits AND, OR, XOR, NOT for bitwise operations (the Intel 8008
#      has native bitwise instructions; the GE-225 has none of these).
#   3. Oct emits LOAD_BYTE / STORE_BYTE / LOAD_ADDR for static variables
#      (the GE-225 is word-addressed with no byte access semantics).
#
# These tests verify that validate_for_ge225() and compile_to_ge225()
# explicitly reject representative Oct IR patterns, ensuring that any attempt
# to route Oct programs through the GE-225 backend fails loudly at compile
# time rather than producing a silently wrong binary.
# ---------------------------------------------------------------------------


class TestOctIrRejected:
    """Verify that Oct-emitted IR opcodes are rejected by the GE-225 backend.

    Each test constructs a minimal IrProgram that contains an opcode Oct uses
    but the GE-225 V1 backend does not support.  The validator must return a
    non-empty error list, and compile_to_ge225 must raise CodeGenError.
    """

    def test_call_rejected(self) -> None:
        """CALL is always emitted by Oct's entry stub (CALL _fn_main).

        Oct's _start always contains CALL _fn_main.  Every single Oct program
        produces a CALL instruction.  The GE-225 V1 backend cannot handle
        subroutine calls (no hardware stack / return-address register), so
        CALL is not in _GE225_SUPPORTED_OPCODES.
        """
        program = _prog(
            _instr(IrOp.LABEL,    IrLabel("_start")),
            _instr(IrOp.LOAD_IMM, _reg(0), _imm(0)),
            _instr(IrOp.CALL,     IrLabel("_fn_main")),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "CALL should be rejected by GE-225 validator"
        assert any("CALL" in e for e in errors)

        with pytest.raises(CodeGenError, match="pre-flight"):
            compile_to_ge225(program)

    def test_ret_rejected(self) -> None:
        """RET is emitted at the end of every Oct function body.

        Every Oct function ends with RET.  The GE-225 has no return
        instruction; it uses indirect BRU for subroutine returns, which the
        V1 backend does not implement.
        """
        program = _prog(
            _instr(IrOp.LABEL, IrLabel("_fn_main")),
            _instr(IrOp.RET),
        )
        errors = validate_for_ge225(program)
        assert errors, "RET should be rejected by GE-225 validator"
        assert any("RET" in e for e in errors)

    def test_and_rejected(self) -> None:
        """AND (register-register) is emitted for Oct's bitwise & operator.

        Oct supports bitwise & on u8 values.  The GE-225 has no register-
        register AND instruction (only AND_IMM 1 for parity extraction is
        implemented in V1).
        """
        program = _prog(
            _instr(IrOp.LABEL, IrLabel("_start")),
            _instr(IrOp.AND, _reg(2), _reg(1), _reg(3)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "AND should be rejected by GE-225 validator"

    def test_or_rejected(self) -> None:
        """OR is emitted for Oct's bitwise | operator.

        The GE-225 has no OR instruction of any kind.
        """
        program = _prog(
            _instr(IrOp.LABEL, IrLabel("_start")),
            _instr(IrOp.OR, _reg(2), _reg(1), _reg(3)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "OR should be rejected by GE-225 validator"

    def test_xor_rejected(self) -> None:
        """XOR is emitted for Oct's bitwise ^ operator.

        The GE-225 has no XOR instruction.
        """
        program = _prog(
            _instr(IrOp.LABEL, IrLabel("_start")),
            _instr(IrOp.XOR, _reg(2), _reg(1), _reg(3)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "XOR should be rejected by GE-225 validator"

    def test_not_rejected(self) -> None:
        """NOT is emitted for Oct's bitwise ~ operator.

        The GE-225 has no NOT instruction.
        """
        program = _prog(
            _instr(IrOp.LABEL, IrLabel("_start")),
            _instr(IrOp.NOT, _reg(2), _reg(1)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "NOT should be rejected by GE-225 validator"

    def test_load_byte_rejected(self) -> None:
        """LOAD_BYTE is emitted for Oct static variable reads.

        Oct's static keyword allocates byte-sized storage.  Reading a static
        emits LOAD_ADDR + LOAD_BYTE.  The GE-225 is word-addressed; there is
        no byte-read instruction.
        """
        program = _prog(
            _instr(IrOp.LABEL,     IrLabel("_start")),
            _instr(IrOp.LOAD_BYTE, _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "LOAD_BYTE should be rejected by GE-225 validator"

    def test_store_byte_rejected(self) -> None:
        """STORE_BYTE is emitted for Oct static variable writes.

        Writing a static variable emits STORE_BYTE.  The GE-225 has no
        byte-write instruction.
        """
        program = _prog(
            _instr(IrOp.LABEL,      IrLabel("_start")),
            _instr(IrOp.STORE_BYTE, _reg(1), _reg(2)),
            _instr(IrOp.HALT),
        )
        errors = validate_for_ge225(program)
        assert errors, "STORE_BYTE should be rejected by GE-225 validator"

    def test_minimal_oct_program_ir_rejected(self) -> None:
        """A minimal Oct 'fn main() {}' IR is rejected by the GE-225 backend.

        Even the simplest possible Oct program — fn main() { } — produces:

            LABEL _start
            LOAD_IMM v0, 0
            CALL _fn_main
            HALT
            LABEL _fn_main
            RET

        Both CALL and RET are in this program, and both are unsupported.
        The validator must report at least those two errors.
        """
        program = IrProgram(entry_label="_start")
        program.add_instruction(_instr(IrOp.LABEL,    IrLabel("_start")))
        program.add_instruction(_instr(IrOp.LOAD_IMM, _reg(0), _imm(0)))
        program.add_instruction(_instr(IrOp.CALL,     IrLabel("_fn_main")))
        program.add_instruction(_instr(IrOp.HALT))
        program.add_instruction(_instr(IrOp.LABEL,    IrLabel("_fn_main")))
        program.add_instruction(_instr(IrOp.RET))

        errors = validate_for_ge225(program)
        assert errors, "Minimal Oct IR must be rejected by GE-225 validator"
        opcodes_mentioned = " ".join(errors)
        assert "CALL" in opcodes_mentioned
        assert "RET" in opcodes_mentioned

        with pytest.raises(CodeGenError, match="pre-flight"):
            compile_to_ge225(program)
