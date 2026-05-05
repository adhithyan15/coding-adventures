"""Tests for the compiler_ir package.

Tests cover:
- IrOp enum values, names, and roundtrip
- Operand types: IrRegister, IrImmediate, IrLabel
- IrInstruction dataclass
- IrDataDecl dataclass
- IrProgram add_instruction / add_data
- IDGenerator: next(), current(), custom start
- print_ir: version, data, entry, labels, comments, regular instructions
- parse_ir: all directive types, roundtrip with print_ir
- IrParseError for malformed input
- Edge cases: empty program, zero-operand instructions, negative immediates
"""

from __future__ import annotations

import pytest

from compiler_ir import (
    NAME_TO_OP,
    OP_NAMES,
    IDGenerator,
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrParseError,
    IrProgram,
    IrRegister,
    parse_ir,
    parse_op,
    print_ir,
)

# =============================================================================
# IrOp tests
# =============================================================================


class TestIrOp:
    """Tests for the IrOp enum."""

    def test_all_opcodes_have_names(self) -> None:
        """Every opcode has a non-empty name."""
        for op in IrOp:
            assert op.name, f"IrOp.{op} has empty name"

    def test_opcode_integer_values(self) -> None:
        """Opcode integer values are stable (regression test)."""
        assert IrOp.LOAD_IMM == 0
        assert IrOp.LOAD_ADDR == 1
        assert IrOp.LOAD_BYTE == 2
        assert IrOp.STORE_BYTE == 3
        assert IrOp.LOAD_WORD == 4
        assert IrOp.STORE_WORD == 5
        assert IrOp.ADD == 6
        assert IrOp.ADD_IMM == 7
        assert IrOp.SUB == 8
        assert IrOp.AND == 9
        assert IrOp.AND_IMM == 10
        assert IrOp.CMP_EQ == 11
        assert IrOp.CMP_NE == 12
        assert IrOp.CMP_LT == 13
        assert IrOp.CMP_GT == 14
        assert IrOp.LABEL == 15
        assert IrOp.JUMP == 16
        assert IrOp.BRANCH_Z == 17
        assert IrOp.BRANCH_NZ == 18
        assert IrOp.CALL == 19
        assert IrOp.RET == 20
        assert IrOp.SYSCALL == 21
        assert IrOp.HALT == 22
        assert IrOp.NOP == 23
        assert IrOp.COMMENT == 24
        assert IrOp.MUL == 25
        assert IrOp.DIV == 26
        assert IrOp.OR == 27
        assert IrOp.OR_IMM == 28
        assert IrOp.XOR == 29
        assert IrOp.XOR_IMM == 30
        assert IrOp.NOT == 31
        assert IrOp.LOAD_F64_IMM == 32
        assert IrOp.LOAD_F64 == 33
        assert IrOp.STORE_F64 == 34
        assert IrOp.F64_ADD == 35
        assert IrOp.F64_SUB == 36
        assert IrOp.F64_MUL == 37
        assert IrOp.F64_DIV == 38
        assert IrOp.F64_CMP_EQ == 39
        assert IrOp.F64_CMP_NE == 40
        assert IrOp.F64_CMP_LT == 41
        assert IrOp.F64_CMP_GT == 42
        assert IrOp.F64_CMP_LE == 43
        assert IrOp.F64_CMP_GE == 44
        assert IrOp.F64_FROM_I32 == 45
        assert IrOp.I32_TRUNC_FROM_F64 == 46
        assert IrOp.MAKE_CLOSURE == 47
        assert IrOp.APPLY_CLOSURE == 48
        assert IrOp.F64_SQRT == 49
        assert IrOp.F64_SIN == 50
        assert IrOp.F64_COS == 51
        assert IrOp.F64_ATAN == 52
        assert IrOp.F64_LN == 53
        assert IrOp.F64_EXP == 54
        assert IrOp.MAKE_CONS == 55
        assert IrOp.CAR == 56
        assert IrOp.CDR == 57
        assert IrOp.IS_NULL == 58
        assert IrOp.IS_PAIR == 59
        assert IrOp.MAKE_SYMBOL == 60
        assert IrOp.IS_SYMBOL == 61
        assert IrOp.LOAD_NIL == 62
        assert IrOp.F64_POW == 63
        assert IrOp.SYSCALL_CHECKED == 64
        assert IrOp.BRANCH_ERR == 65
        assert IrOp.THROW == 66
        # VMCOND00 Phase 3 — Layer 3 dynamic handler opcodes
        assert IrOp.PUSH_HANDLER == 67
        assert IrOp.POP_HANDLER == 68
        assert IrOp.SIGNAL == 69
        assert IrOp.ERROR == 70
        assert IrOp.WARN == 71

    def test_total_opcode_count(self) -> None:
        """There are exactly 72 opcodes after VMCOND00 Phase 3."""
        assert len(IrOp) == 72

    def test_name_to_op_roundtrip(self) -> None:
        """NAME_TO_OP[op.name] == op for every opcode."""
        for op in IrOp:
            assert NAME_TO_OP[op.name] == op

    def test_op_names_roundtrip(self) -> None:
        """OP_NAMES[op] == op.name for every opcode."""
        for op in IrOp:
            assert OP_NAMES[op] == op.name

    def test_parse_op_known(self) -> None:
        """parse_op returns the correct IrOp for known names."""
        assert parse_op("ADD_IMM") == IrOp.ADD_IMM
        assert parse_op("HALT") == IrOp.HALT
        assert parse_op("BRANCH_Z") == IrOp.BRANCH_Z
        assert parse_op("LOAD_BYTE") == IrOp.LOAD_BYTE

    def test_parse_op_unknown(self) -> None:
        """parse_op returns None for unknown names."""
        assert parse_op("FROBNITZ") is None
        assert parse_op("") is None
        assert parse_op("add_imm") is None  # case-sensitive


# =============================================================================
# Operand types
# =============================================================================


class TestIrRegister:
    """Tests for IrRegister operands."""

    def test_str_v0(self) -> None:
        assert str(IrRegister(0)) == "v0"

    def test_str_v5(self) -> None:
        assert str(IrRegister(5)) == "v5"

    def test_str_large_index(self) -> None:
        assert str(IrRegister(65535)) == "v65535"

    def test_equality(self) -> None:
        assert IrRegister(3) == IrRegister(3)
        assert IrRegister(3) != IrRegister(4)

    def test_frozen(self) -> None:
        """IrRegister is immutable (frozen dataclass)."""
        r = IrRegister(0)
        with pytest.raises(Exception):
            r.index = 1  # type: ignore[misc]

    def test_hash(self) -> None:
        """IrRegister is hashable (frozen dataclass)."""
        s = {IrRegister(0), IrRegister(1), IrRegister(0)}
        assert len(s) == 2


class TestIrImmediate:
    """Tests for IrImmediate operands."""

    def test_str_positive(self) -> None:
        assert str(IrImmediate(42)) == "42"

    def test_str_negative(self) -> None:
        assert str(IrImmediate(-1)) == "-1"

    def test_str_zero(self) -> None:
        assert str(IrImmediate(0)) == "0"

    def test_str_large(self) -> None:
        assert str(IrImmediate(255)) == "255"

    def test_equality(self) -> None:
        assert IrImmediate(10) == IrImmediate(10)
        assert IrImmediate(10) != IrImmediate(11)

    def test_frozen(self) -> None:
        imm = IrImmediate(5)
        with pytest.raises(Exception):
            imm.value = 6  # type: ignore[misc]


class TestIrLabel:
    """Tests for IrLabel operands."""

    def test_str(self) -> None:
        assert str(IrLabel("_start")) == "_start"
        assert str(IrLabel("loop_0_end")) == "loop_0_end"
        assert str(IrLabel("tape")) == "tape"

    def test_equality(self) -> None:
        assert IrLabel("tape") == IrLabel("tape")
        assert IrLabel("tape") != IrLabel("tape2")

    def test_frozen(self) -> None:
        lbl = IrLabel("x")
        with pytest.raises(Exception):
            lbl.name = "y"  # type: ignore[misc]


class TestIrFloatImmediate:
    """Tests for IrFloatImmediate operands."""

    def test_str_positive(self) -> None:
        assert str(IrFloatImmediate(1.5)) == "1.5"

    def test_str_negative(self) -> None:
        assert str(IrFloatImmediate(-0.25)) == "-0.25"

    def test_equality(self) -> None:
        assert IrFloatImmediate(3.5) == IrFloatImmediate(3.5)
        assert IrFloatImmediate(3.5) != IrFloatImmediate(4.5)

    def test_frozen(self) -> None:
        imm = IrFloatImmediate(2.5)
        with pytest.raises(Exception):
            imm.value = 6.0  # type: ignore[misc]


# =============================================================================
# IrInstruction
# =============================================================================


class TestIrInstruction:
    """Tests for IrInstruction dataclass."""

    def test_default_id_is_minus_one(self) -> None:
        instr = IrInstruction(opcode=IrOp.HALT)
        assert instr.id == -1

    def test_default_operands_empty(self) -> None:
        instr = IrInstruction(opcode=IrOp.HALT)
        assert instr.operands == []

    def test_full_construction(self) -> None:
        instr = IrInstruction(
            opcode=IrOp.ADD_IMM,
            operands=[IrRegister(1), IrRegister(1), IrImmediate(1)],
            id=7,
        )
        assert instr.opcode == IrOp.ADD_IMM
        assert len(instr.operands) == 3
        assert instr.id == 7

    def test_mutable(self) -> None:
        """IrInstruction is a regular (mutable) dataclass."""
        instr = IrInstruction(opcode=IrOp.NOP, id=0)
        instr.id = 99
        assert instr.id == 99


# =============================================================================
# IrDataDecl
# =============================================================================


class TestIrDataDecl:
    """Tests for IrDataDecl dataclass."""

    def test_tape_decl(self) -> None:
        d = IrDataDecl(label="tape", size=30000, init=0)
        assert d.label == "tape"
        assert d.size == 30000
        assert d.init == 0

    def test_default_init(self) -> None:
        d = IrDataDecl(label="buf", size=1024)
        assert d.init == 0

    def test_custom_init(self) -> None:
        d = IrDataDecl(label="ones", size=16, init=255)
        assert d.init == 255


# =============================================================================
# IrProgram
# =============================================================================


class TestIrProgram:
    """Tests for IrProgram."""

    def test_default_version(self) -> None:
        prog = IrProgram(entry_label="_start")
        assert prog.version == 1

    def test_empty_instructions(self) -> None:
        prog = IrProgram(entry_label="_start")
        assert prog.instructions == []

    def test_empty_data(self) -> None:
        prog = IrProgram(entry_label="_start")
        assert prog.data == []

    def test_add_instruction(self) -> None:
        prog = IrProgram(entry_label="_start")
        instr = IrInstruction(opcode=IrOp.HALT, id=0)
        prog.add_instruction(instr)
        assert len(prog.instructions) == 1
        assert prog.instructions[0].opcode == IrOp.HALT

    def test_add_data(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl("tape", 30000, 0))
        assert len(prog.data) == 1
        assert prog.data[0].label == "tape"

    def test_multiple_instructions(self) -> None:
        prog = IrProgram(entry_label="_start")
        for i in range(5):
            prog.add_instruction(IrInstruction(opcode=IrOp.NOP, id=i))
        assert len(prog.instructions) == 5


# =============================================================================
# IDGenerator
# =============================================================================


class TestIDGenerator:
    """Tests for IDGenerator."""

    def test_starts_at_zero(self) -> None:
        gen = IDGenerator()
        assert gen.next() == 0

    def test_increments(self) -> None:
        gen = IDGenerator()
        assert gen.next() == 0
        assert gen.next() == 1
        assert gen.next() == 2

    def test_current_before_next(self) -> None:
        gen = IDGenerator()
        assert gen.current() == 0

    def test_current_after_next(self) -> None:
        gen = IDGenerator()
        gen.next()
        assert gen.current() == 1

    def test_custom_start(self) -> None:
        gen = IDGenerator(start=100)
        assert gen.next() == 100
        assert gen.next() == 101

    def test_uniqueness(self) -> None:
        """Every ID returned by next() is unique."""
        gen = IDGenerator()
        ids = [gen.next() for _ in range(1000)]
        assert len(set(ids)) == 1000


# =============================================================================
# print_ir
# =============================================================================


class TestPrintIr:
    """Tests for print_ir()."""

    def _make_minimal_program(self) -> IrProgram:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(opcode=IrOp.HALT, id=0))
        return prog

    def test_version_directive_present(self) -> None:
        prog = self._make_minimal_program()
        text = print_ir(prog)
        assert ".version 1" in text

    def test_entry_directive_present(self) -> None:
        prog = self._make_minimal_program()
        text = print_ir(prog)
        assert ".entry _start" in text

    def test_halt_instruction_present(self) -> None:
        prog = self._make_minimal_program()
        text = print_ir(prog)
        assert "HALT" in text

    def test_halt_has_id_comment(self) -> None:
        prog = self._make_minimal_program()
        text = print_ir(prog)
        assert "; #0" in text

    def test_data_directive(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl("tape", 30000, 0))
        text = print_ir(prog)
        assert ".data tape 30000 0" in text

    def test_label_instruction(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LABEL,
            operands=[IrLabel("_start")],
            id=-1,
        ))
        text = print_ir(prog)
        assert "_start:" in text

    def test_comment_instruction(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.COMMENT,
            operands=[IrLabel("load tape base address")],
            id=-1,
        ))
        text = print_ir(prog)
        assert "; load tape base address" in text

    def test_add_imm_instruction(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.ADD_IMM,
            operands=[IrRegister(1), IrRegister(1), IrImmediate(1)],
            id=3,
        ))
        text = print_ir(prog)
        assert "ADD_IMM" in text
        assert "v1, v1, 1" in text
        assert "; #3" in text

    def test_negative_immediate(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.ADD_IMM,
            operands=[IrRegister(1), IrRegister(1), IrImmediate(-1)],
            id=0,
        ))
        text = print_ir(prog)
        assert "-1" in text

    def test_instruction_indented_two_spaces(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(opcode=IrOp.HALT, id=0))
        text = print_ir(prog)
        for line in text.splitlines():
            if "HALT" in line:
                assert line.startswith("  "), f"Expected 2-space indent: {line!r}"

    def test_label_not_indented(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LABEL,
            operands=[IrLabel("loop_0_start")],
            id=-1,
        ))
        text = print_ir(prog)
        for line in text.splitlines():
            if "loop_0_start:" in line:
                assert not line.startswith(" "), f"Label should not be indented: {line!r}"

    def test_version_is_first_line(self) -> None:
        prog = self._make_minimal_program()
        text = print_ir(prog)
        assert text.startswith(".version ")

    def test_multiple_data_declarations(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl("tape", 30000, 0))
        prog.add_data(IrDataDecl("buf", 256, 0))
        text = print_ir(prog)
        assert ".data tape 30000 0" in text
        assert ".data buf 256 0" in text

    def test_empty_comment_instruction(self) -> None:
        """COMMENT with no operands prints '  ; '."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(opcode=IrOp.COMMENT, id=-1))
        text = print_ir(prog)
        assert "  ; " in text

    def test_load_addr_instruction(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LOAD_ADDR,
            operands=[IrRegister(0), IrLabel("tape")],
            id=0,
        ))
        text = print_ir(prog)
        assert "LOAD_ADDR" in text
        assert "v0, tape" in text


# =============================================================================
# parse_ir
# =============================================================================


class TestParseIr:
    """Tests for parse_ir()."""

    def test_version_parsed(self) -> None:
        prog = parse_ir(".version 1\n.entry _start\n")
        assert prog.version == 1

    def test_version_2(self) -> None:
        prog = parse_ir(".version 2\n.entry _start\n")
        assert prog.version == 2

    def test_entry_parsed(self) -> None:
        prog = parse_ir(".version 1\n.entry _start\n")
        assert prog.entry_label == "_start"

    def test_data_parsed(self) -> None:
        prog = parse_ir(".version 1\n.data tape 30000 0\n.entry _start\n")
        assert len(prog.data) == 1
        d = prog.data[0]
        assert d.label == "tape"
        assert d.size == 30000
        assert d.init == 0

    def test_label_instruction_parsed(self) -> None:
        text = ".version 1\n.entry _start\n\n_start:\n"
        prog = parse_ir(text)
        assert len(prog.instructions) == 1
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.LABEL
        assert len(instr.operands) == 1
        assert isinstance(instr.operands[0], IrLabel)
        assert instr.operands[0].name == "_start"
        assert instr.id == -1

    def test_halt_instruction_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  HALT                  ; #0\n"
        prog = parse_ir(text)
        assert len(prog.instructions) == 1
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.HALT
        assert instr.id == 0

    def test_add_imm_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  ADD_IMM    v1, v1, 1  ; #3\n"
        prog = parse_ir(text)
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.ADD_IMM
        assert instr.operands == [IrRegister(1), IrRegister(1), IrImmediate(1)]
        assert instr.id == 3

    def test_negative_immediate_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  ADD_IMM    v1, v1, -1  ; #5\n"
        prog = parse_ir(text)
        instr = prog.instructions[0]
        assert instr.operands[2] == IrImmediate(-1)

    def test_float_immediate_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  LOAD_F64_IMM v2, 1.5 ; #7\n"
        prog = parse_ir(text)
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.LOAD_F64_IMM
        assert instr.operands == [IrRegister(2), IrFloatImmediate(1.5)]

    def test_label_operand_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  JUMP       loop_0_start  ; #7\n"
        prog = parse_ir(text)
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.JUMP
        assert instr.operands == [IrLabel("loop_0_start")]

    def test_comment_line_parsed(self) -> None:
        text = ".version 1\n.entry _start\n  ; load tape base address\n"
        prog = parse_ir(text)
        assert len(prog.instructions) == 1
        instr = prog.instructions[0]
        assert instr.opcode == IrOp.COMMENT
        assert instr.operands[0].name == "load tape base address"

    def test_blank_lines_skipped(self) -> None:
        text = ".version 1\n\n\n.entry _start\n\n  HALT  ; #0\n"
        prog = parse_ir(text)
        assert len(prog.instructions) == 1

    def test_unknown_opcode_raises(self) -> None:
        text = ".version 1\n.entry _start\n  FROBNITZ v0  ; #0\n"
        with pytest.raises(IrParseError, match="unknown opcode"):
            parse_ir(text)

    def test_invalid_version_raises(self) -> None:
        with pytest.raises(IrParseError):
            parse_ir(".version notanumber\n.entry _start\n")

    def test_invalid_data_size_raises(self) -> None:
        with pytest.raises(IrParseError):
            parse_ir(".version 1\n.data tape notanumber 0\n.entry _start\n")

    def test_invalid_data_init_raises(self) -> None:
        with pytest.raises(IrParseError):
            parse_ir(".version 1\n.data tape 1024 notanumber\n.entry _start\n")

    def test_too_many_data_fields_raises(self) -> None:
        with pytest.raises(IrParseError):
            parse_ir(".version 1\n.data tape 1024 0 extra\n.entry _start\n")

    def test_too_few_data_fields_raises(self) -> None:
        with pytest.raises(IrParseError):
            parse_ir(".version 1\n.data tape 1024\n.entry _start\n")

    def test_multiple_instructions_parsed(self) -> None:
        text = (
            ".version 1\n"
            ".data tape 30000 0\n"
            ".entry _start\n"
            "\n"
            "_start:\n"
            "  LOAD_ADDR   v0, tape          ; #0\n"
            "  LOAD_IMM    v1, 0             ; #1\n"
            "  HALT                          ; #2\n"
        )
        prog = parse_ir(text)
        # 1 LABEL + 3 regular instructions
        assert len(prog.instructions) == 4
        assert prog.instructions[0].opcode == IrOp.LABEL
        assert prog.instructions[1].opcode == IrOp.LOAD_ADDR
        assert prog.instructions[2].opcode == IrOp.LOAD_IMM
        assert prog.instructions[3].opcode == IrOp.HALT


# =============================================================================
# Roundtrip tests (print_ir → parse_ir → print_ir)
# =============================================================================


class TestRoundtrip:
    """Tests that parse(print(prog)) produces an equivalent program."""

    def _make_brainfuck_program(self) -> IrProgram:
        """Build a representative Brainfuck IR program."""
        gen = IDGenerator()
        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl("tape", 30000, 0))

        prog.add_instruction(IrInstruction(
            opcode=IrOp.LABEL,
            operands=[IrLabel("_start")],
            id=-1,
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LOAD_ADDR,
            operands=[IrRegister(0), IrLabel("tape")],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LOAD_IMM,
            operands=[IrRegister(1), IrImmediate(0)],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LABEL,
            operands=[IrLabel("loop_0_start")],
            id=-1,
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LOAD_BYTE,
            operands=[IrRegister(2), IrRegister(0), IrRegister(1)],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.BRANCH_Z,
            operands=[IrRegister(2), IrLabel("loop_0_end")],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.ADD_IMM,
            operands=[IrRegister(2), IrRegister(2), IrImmediate(-1)],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.AND_IMM,
            operands=[IrRegister(2), IrRegister(2), IrImmediate(255)],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.STORE_BYTE,
            operands=[IrRegister(2), IrRegister(0), IrRegister(1)],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.JUMP,
            operands=[IrLabel("loop_0_start")],
            id=gen.next(),
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.LABEL,
            operands=[IrLabel("loop_0_end")],
            id=-1,
        ))
        prog.add_instruction(IrInstruction(
            opcode=IrOp.HALT,
            operands=[],
            id=gen.next(),
        ))
        return prog

    def test_instruction_count_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        assert len(parsed.instructions) == len(prog.instructions)

    def test_opcodes_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        for orig, roundtripped in zip(prog.instructions, parsed.instructions):
            assert orig.opcode == roundtripped.opcode

    def test_operand_count_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        for orig, roundtripped in zip(prog.instructions, parsed.instructions):
            assert len(orig.operands) == len(roundtripped.operands)

    def test_data_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        assert len(parsed.data) == len(prog.data)
        for d_orig, d_parsed in zip(prog.data, parsed.data):
            assert d_orig.label == d_parsed.label
            assert d_orig.size == d_parsed.size
            assert d_orig.init == d_parsed.init

    def test_entry_label_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        assert parsed.entry_label == prog.entry_label

    def test_version_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        assert parsed.version == prog.version

    def test_ids_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        for orig, rt in zip(prog.instructions, parsed.instructions):
            assert orig.id == rt.id

    def test_operand_types_preserved(self) -> None:
        prog = self._make_brainfuck_program()
        text = print_ir(prog)
        parsed = parse_ir(text)
        for orig, rt in zip(prog.instructions, parsed.instructions):
            for o_orig, o_rt in zip(orig.operands, rt.operands):
                assert type(o_orig) == type(o_rt)
                assert str(o_orig) == str(o_rt)

    def test_print_parse_print_stable(self) -> None:
        """Second print of a parsed program equals first print."""
        prog = self._make_brainfuck_program()
        text1 = print_ir(prog)
        parsed = parse_ir(text1)
        text2 = print_ir(parsed)
        assert text1 == text2


# =============================================================================
# Bitwise opcode tests (OR, OR_IMM, XOR, XOR_IMM, NOT)
# =============================================================================


class TestBitwiseOpcodes:
    """Tests for the five new bitwise IR opcodes added in v0.2.0.

    Each opcode is verified for:
    - Correct IntEnum value (stability regression)
    - parse_op() round-trip
    - print_ir() / parse_ir() round-trip (text format)
    """

    # ── OR ────────────────────────────────────────────────────────────────────

    def test_or_integer_value(self) -> None:
        """IrOp.OR == 27 (stable integer, never changes)."""
        assert IrOp.OR == 27

    def test_or_name(self) -> None:
        assert IrOp.OR.name == "OR"

    def test_or_parse_op(self) -> None:
        assert parse_op("OR") == IrOp.OR

    def test_or_print_parse_roundtrip(self) -> None:
        """OR instruction prints and parses correctly."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.OR,
            operands=[IrRegister(3), IrRegister(1), IrRegister(2)],
            id=0,
        ))
        text = print_ir(prog)
        assert "OR" in text
        parsed = parse_ir(text)
        instr = parsed.instructions[0]
        assert instr.opcode == IrOp.OR
        assert instr.operands == [IrRegister(3), IrRegister(1), IrRegister(2)]

    # ── OR_IMM ────────────────────────────────────────────────────────────────

    def test_or_imm_integer_value(self) -> None:
        assert IrOp.OR_IMM == 28

    def test_or_imm_name(self) -> None:
        assert IrOp.OR_IMM.name == "OR_IMM"

    def test_or_imm_parse_op(self) -> None:
        assert parse_op("OR_IMM") == IrOp.OR_IMM

    def test_or_imm_print_parse_roundtrip(self) -> None:
        """OR_IMM with immediate 0x80 round-trips through text."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.OR_IMM,
            operands=[IrRegister(2), IrRegister(2), IrImmediate(0x80)],
            id=0,
        ))
        text = print_ir(prog)
        assert "OR_IMM" in text
        assert "128" in text  # 0x80 = 128
        parsed = parse_ir(text)
        instr = parsed.instructions[0]
        assert instr.opcode == IrOp.OR_IMM
        assert instr.operands[2] == IrImmediate(128)

    # ── XOR ───────────────────────────────────────────────────────────────────

    def test_xor_integer_value(self) -> None:
        assert IrOp.XOR == 29

    def test_xor_name(self) -> None:
        assert IrOp.XOR.name == "XOR"

    def test_xor_parse_op(self) -> None:
        assert parse_op("XOR") == IrOp.XOR

    def test_xor_print_parse_roundtrip(self) -> None:
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.XOR,
            operands=[IrRegister(3), IrRegister(1), IrRegister(2)],
            id=0,
        ))
        text = print_ir(prog)
        assert "XOR" in text
        parsed = parse_ir(text)
        instr = parsed.instructions[0]
        assert instr.opcode == IrOp.XOR
        assert instr.operands == [IrRegister(3), IrRegister(1), IrRegister(2)]

    # ── XOR_IMM ───────────────────────────────────────────────────────────────

    def test_xor_imm_integer_value(self) -> None:
        assert IrOp.XOR_IMM == 30

    def test_xor_imm_name(self) -> None:
        assert IrOp.XOR_IMM.name == "XOR_IMM"

    def test_xor_imm_parse_op(self) -> None:
        assert parse_op("XOR_IMM") == IrOp.XOR_IMM

    def test_xor_imm_print_parse_roundtrip(self) -> None:
        """XOR_IMM 0xFF is the canonical NOT-a-byte idiom."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.XOR_IMM,
            operands=[IrRegister(2), IrRegister(2), IrImmediate(0xFF)],
            id=0,
        ))
        text = print_ir(prog)
        assert "XOR_IMM" in text
        assert "255" in text
        parsed = parse_ir(text)
        instr = parsed.instructions[0]
        assert instr.opcode == IrOp.XOR_IMM
        assert instr.operands[2] == IrImmediate(255)

    # ── NOT ───────────────────────────────────────────────────────────────────

    def test_not_integer_value(self) -> None:
        assert IrOp.NOT == 31

    def test_not_name(self) -> None:
        assert IrOp.NOT.name == "NOT"

    def test_not_parse_op(self) -> None:
        assert parse_op("NOT") == IrOp.NOT

    def test_not_print_parse_roundtrip(self) -> None:
        """NOT is a 2-operand instruction: NOT dst, src."""
        prog = IrProgram(entry_label="_start")
        prog.add_instruction(IrInstruction(
            opcode=IrOp.NOT,
            operands=[IrRegister(2), IrRegister(1)],
            id=0,
        ))
        text = print_ir(prog)
        assert "NOT" in text
        parsed = parse_ir(text)
        instr = parsed.instructions[0]
        assert instr.opcode == IrOp.NOT
        assert instr.operands == [IrRegister(2), IrRegister(1)]

    def test_not_two_operands_only(self) -> None:
        """NOT takes exactly dst and src — no immediate."""
        instr = IrInstruction(
            opcode=IrOp.NOT,
            operands=[IrRegister(0), IrRegister(1)],
            id=0,
        )
        assert len(instr.operands) == 2
        assert isinstance(instr.operands[0], IrRegister)
        assert isinstance(instr.operands[1], IrRegister)

    # ── Cross-cutting ─────────────────────────────────────────────────────────

    def test_all_five_in_name_to_op(self) -> None:
        """All five new opcodes are reachable via NAME_TO_OP."""
        for name in ("OR", "OR_IMM", "XOR", "XOR_IMM", "NOT"):
            assert name in NAME_TO_OP, f"{name} missing from NAME_TO_OP"

    def test_all_five_in_op_names(self) -> None:
        """All five new opcodes are reachable via OP_NAMES."""
        for op in (IrOp.OR, IrOp.OR_IMM, IrOp.XOR, IrOp.XOR_IMM, IrOp.NOT):
            assert op in OP_NAMES, f"{op} missing from OP_NAMES"
            assert OP_NAMES[op] == op.name

    def test_new_opcodes_are_distinct(self) -> None:
        """No two of the five new opcodes share an integer value."""
        new_ops = (IrOp.OR, IrOp.OR_IMM, IrOp.XOR, IrOp.XOR_IMM, IrOp.NOT)
        assert len({int(op) for op in new_ops}) == 5

    def test_new_opcodes_do_not_collide_with_existing(self) -> None:
        """The five new integer values are not used by any pre-existing opcode."""
        pre_existing = {int(op) for op in IrOp} - {27, 28, 29, 30, 31}
        for v in (27, 28, 29, 30, 31):
            assert v not in pre_existing


# =============================================================================
# All opcodes can be printed and parsed
# =============================================================================


class TestAllOpcodesPrintParse:
    """Ensure every opcode can survive a print/parse roundtrip."""

    def test_all_opcodes_roundtrip(self) -> None:
        """Build one instruction per opcode and verify roundtrip."""
        prog = IrProgram(entry_label="_start")
        operands_by_opcode = {
            IrOp.LOAD_IMM:   [IrRegister(0), IrImmediate(42)],
            IrOp.LOAD_ADDR:  [IrRegister(0), IrLabel("tape")],
            IrOp.LOAD_BYTE:  [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.STORE_BYTE: [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.LOAD_WORD:  [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.STORE_WORD: [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.ADD:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.ADD_IMM:    [IrRegister(1), IrRegister(1), IrImmediate(1)],
            IrOp.SUB:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.AND:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.AND_IMM:    [IrRegister(2), IrRegister(2), IrImmediate(255)],
            IrOp.CMP_EQ:     [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.CMP_NE:     [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.CMP_LT:     [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.CMP_GT:     [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.LABEL:      [IrLabel("test_label")],
            IrOp.JUMP:       [IrLabel("test_label")],
            IrOp.BRANCH_Z:   [IrRegister(2), IrLabel("test_label")],
            IrOp.BRANCH_NZ:  [IrRegister(2), IrLabel("test_label")],
            IrOp.CALL:       [IrLabel("my_func")],
            IrOp.RET:        [],
            IrOp.SYSCALL:    [IrImmediate(1)],
            IrOp.HALT:       [],
            IrOp.NOP:        [],
            IrOp.COMMENT:    [IrLabel("a test comment")],
            IrOp.MUL:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.DIV:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.OR:         [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.OR_IMM:     [IrRegister(2), IrRegister(2), IrImmediate(0x80)],
            IrOp.XOR:        [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.XOR_IMM:    [IrRegister(2), IrRegister(2), IrImmediate(0xFF)],
            IrOp.NOT:        [IrRegister(2), IrRegister(1)],
            IrOp.LOAD_F64_IMM: [IrRegister(2), IrFloatImmediate(1.5)],
            IrOp.LOAD_F64:     [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.STORE_F64:    [IrRegister(2), IrRegister(0), IrRegister(1)],
            IrOp.F64_ADD:      [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.F64_SUB:      [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.F64_MUL:      [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.F64_DIV:      [IrRegister(3), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_EQ:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_NE:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_LT:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_GT:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_LE:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_CMP_GE:   [IrRegister(4), IrRegister(1), IrRegister(2)],
            IrOp.F64_FROM_I32: [IrRegister(2), IrRegister(1)],
            IrOp.I32_TRUNC_FROM_F64: [IrRegister(2), IrRegister(1)],
            IrOp.F64_SQRT:     [IrRegister(2), IrRegister(1)],
            IrOp.F64_SIN:      [IrRegister(2), IrRegister(1)],
            IrOp.F64_COS:      [IrRegister(2), IrRegister(1)],
            IrOp.F64_ATAN:     [IrRegister(2), IrRegister(1)],
            IrOp.F64_LN:       [IrRegister(2), IrRegister(1)],
            IrOp.F64_EXP:      [IrRegister(2), IrRegister(1)],
            IrOp.F64_POW:      [IrRegister(3), IrRegister(1), IrRegister(2)],
            # MAKE_CLOSURE dst, fn_label, num_captured, capt0, capt1
            IrOp.MAKE_CLOSURE: [
                IrRegister(0),
                IrLabel("_lambda_0"),
                IrImmediate(2),
                IrRegister(1),
                IrRegister(2),
            ],
            # APPLY_CLOSURE dst, closure_reg, num_args, arg0
            IrOp.APPLY_CLOSURE: [
                IrRegister(0),
                IrRegister(3),
                IrImmediate(1),
                IrRegister(4),
            ],
            # MAKE_CONS dst, head_reg, tail_reg
            IrOp.MAKE_CONS:    [IrRegister(5), IrRegister(1), IrRegister(2)],
            # CAR dst, src
            IrOp.CAR:          [IrRegister(2), IrRegister(5)],
            # CDR dst, src
            IrOp.CDR:          [IrRegister(2), IrRegister(5)],
            # IS_NULL dst, src
            IrOp.IS_NULL:      [IrRegister(2), IrRegister(5)],
            # IS_PAIR dst, src
            IrOp.IS_PAIR:      [IrRegister(2), IrRegister(5)],
            # MAKE_SYMBOL dst, name_label
            IrOp.MAKE_SYMBOL:  [IrRegister(2), IrLabel("foo")],
            # IS_SYMBOL dst, src
            IrOp.IS_SYMBOL:    [IrRegister(2), IrRegister(5)],
            # LOAD_NIL dst
            IrOp.LOAD_NIL:     [IrRegister(2)],
            # SYSCALL_CHECKED n, arg_reg, val_dst, err_dst
            IrOp.SYSCALL_CHECKED: [
                IrImmediate(2),    # n = read-byte
                IrRegister(0),     # arg (ignored for read-byte)
                IrRegister(1),     # val_dst — byte read
                IrRegister(2),     # err_dst — error code
            ],
            # BRANCH_ERR err_reg, label
            IrOp.BRANCH_ERR:   [IrRegister(2), IrLabel("eof_handler")],
            # THROW condition_reg
            IrOp.THROW:        [IrRegister(0)],
            # VMCOND00 Phase 3 — PUSH_HANDLER type_id:label fn:reg
            IrOp.PUSH_HANDLER: [IrLabel("*"), IrRegister(1)],
            # POP_HANDLER (no operands)
            IrOp.POP_HANDLER:  [],
            # SIGNAL condition:reg
            IrOp.SIGNAL:       [IrRegister(0)],
            # ERROR condition:reg
            IrOp.ERROR:        [IrRegister(0)],
            # WARN condition:reg
            IrOp.WARN:         [IrRegister(0)],
        }
        for idx, op in enumerate(IrOp):
            operands = operands_by_opcode[op]
            prog.add_instruction(IrInstruction(opcode=op, operands=operands, id=idx))

        text = print_ir(prog)
        parsed = parse_ir(text)
        assert len(parsed.instructions) == len(prog.instructions)
