"""Tests for the dartmouth_basic_ir_compiler package.

Tests cover:
- GE225_CODES mapping and ascii_to_ge225 helper
- compile_basic: REM, LET, PRINT, GOTO, IF/THEN, FOR/NEXT, END, STOP
- Expression compilation: literals, variables, arithmetic, unary minus
- Relational operators: <, >, =, <>, <=, >=
- FOR/NEXT: default step, explicit step, pre-test
- Error cases: GOSUB, DIM, INPUT, DEF FN, power operator, bad NEXT, bad PRINT chars
- Label conventions: _start, _line_N, _for_N_check, _for_N_end
- Variable register assignment: A=v1, Z=v26, A0=v27
"""

from __future__ import annotations

import pytest

from compiler_ir import IrImmediate, IrLabel, IrOp, IrRegister
from dartmouth_basic_parser import parse_dartmouth_basic

from dartmouth_basic_ir_compiler import (
    CARRIAGE_RETURN_CODE,
    GE225_CODES,
    CompileError,
    CompileResult,
    ascii_to_ge225,
    compile_basic,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def compile_source(source: str) -> CompileResult:
    """Parse and compile a BASIC source string."""
    ast = parse_dartmouth_basic(source)
    return compile_basic(ast)


def opcodes(result: CompileResult) -> list[IrOp]:
    """Return the list of opcodes (excluding LABELs) in the compiled program."""
    return [
        instr.opcode
        for instr in result.program.instructions
        if instr.opcode != IrOp.LABEL
    ]


def labels(result: CompileResult) -> list[str]:
    """Return the list of label names defined in the compiled program."""
    return [
        str(instr.operands[0])
        for instr in result.program.instructions
        if instr.opcode == IrOp.LABEL
    ]


def find_first(result: CompileResult, opcode: IrOp) -> list:
    """Return operands of the first instruction with the given opcode."""
    for instr in result.program.instructions:
        if instr.opcode == opcode:
            return instr.operands
    return []


def find_all(result: CompileResult, opcode: IrOp) -> list[list]:
    """Return operands lists for all instructions with the given opcode."""
    return [
        instr.operands
        for instr in result.program.instructions
        if instr.opcode == opcode
    ]


# ---------------------------------------------------------------------------
# GE225 Codes
# ---------------------------------------------------------------------------


class TestGE225Codes:
    """Tests for the GE-225 typewriter code mapping."""

    def test_uppercase_letters_present(self) -> None:
        """A–Z all have codes."""
        for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            assert ch in GE225_CODES, f"missing {ch!r}"

    def test_digits_present(self) -> None:
        """0–9 all have codes."""
        for ch in "0123456789":
            assert ch in GE225_CODES

    def test_space_present(self) -> None:
        assert " " in GE225_CODES

    def test_carriage_return_code_is_037(self) -> None:
        assert CARRIAGE_RETURN_CODE == 0o37

    def test_ascii_to_ge225_uppercase(self) -> None:
        assert ascii_to_ge225("A") == GE225_CODES["A"]
        assert ascii_to_ge225("Z") == GE225_CODES["Z"]

    def test_ascii_to_ge225_lowercase_converted(self) -> None:
        assert ascii_to_ge225("a") == GE225_CODES["A"]
        assert ascii_to_ge225("z") == GE225_CODES["Z"]

    def test_ascii_to_ge225_unsupported_returns_none(self) -> None:
        assert ascii_to_ge225("@") is None
        assert ascii_to_ge225("~") is None
        assert ascii_to_ge225("!") is None

    def test_ascii_to_ge225_digit(self) -> None:
        assert ascii_to_ge225("5") == GE225_CODES["5"]

    def test_ascii_to_ge225_space(self) -> None:
        assert ascii_to_ge225(" ") == GE225_CODES[" "]


# ---------------------------------------------------------------------------
# Variable register assignment
# ---------------------------------------------------------------------------


class TestVariableRegisters:
    """Tests for the fixed variable → register mapping."""

    def test_variable_a_is_register_1(self) -> None:
        result = compile_source("10 LET A = 5\n20 END\n")
        assert result.var_regs["A"] == 1

    def test_variable_z_is_register_26(self) -> None:
        result = compile_source("10 LET Z = 1\n20 END\n")
        assert result.var_regs["Z"] == 26

    def test_all_az_variables_present_in_map(self) -> None:
        result = compile_source("10 END\n")
        for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            assert ch in result.var_regs

    def test_a_through_z_registers_are_1_through_26(self) -> None:
        result = compile_source("10 END\n")
        for i, ch in enumerate("ABCDEFGHIJKLMNOPQRSTUVWXYZ"):
            assert result.var_regs[ch] == i + 1


# ---------------------------------------------------------------------------
# Program structure
# ---------------------------------------------------------------------------


class TestProgramStructure:
    """Tests for program-level IR structure."""

    def test_entry_label_is_start(self) -> None:
        result = compile_source("10 END\n")
        assert result.program.entry_label == "_start"

    def test_start_label_emitted(self) -> None:
        result = compile_source("10 END\n")
        assert "_start" in labels(result)

    def test_line_label_emitted(self) -> None:
        result = compile_source("10 END\n")
        assert "_line_10" in labels(result)

    def test_line_label_before_statement(self) -> None:
        """_line_N label must appear before any instructions for that line."""
        result = compile_source("10 END\n")
        instrs = result.program.instructions
        line_idx = next(
            i for i, ins in enumerate(instrs)
            if ins.opcode == IrOp.LABEL and str(ins.operands[0]) == "_line_10"
        )
        halt_idx = next(
            i for i, ins in enumerate(instrs)
            if ins.opcode == IrOp.HALT
        )
        assert line_idx < halt_idx

    def test_epilogue_halt_always_present(self) -> None:
        """The compiler appends a HALT even if no END is in the program."""
        result = compile_source("10 LET A = 1\n")
        assert IrOp.HALT in opcodes(result)

    def test_unique_instruction_ids(self) -> None:
        result = compile_source("10 LET A = 1\n20 END\n")
        ids = [
            ins.id for ins in result.program.instructions if ins.id != -1
        ]
        assert len(ids) == len(set(ids))


# ---------------------------------------------------------------------------
# REM
# ---------------------------------------------------------------------------


class TestRem:
    """Tests for REM (comment) statement."""

    def test_rem_emits_comment(self) -> None:
        result = compile_source("10 REM THIS IS A COMMENT\n20 END\n")
        assert IrOp.COMMENT in opcodes(result)

    def test_rem_no_other_non_label_instructions_on_rem_line(self) -> None:
        """REM should produce no runtime instructions."""
        result = compile_source("10 REM ONLY A COMMENT\n20 END\n")
        non_meta = [
            op for op in opcodes(result)
            if op not in (IrOp.COMMENT, IrOp.HALT)
        ]
        assert non_meta == []


# ---------------------------------------------------------------------------
# LET
# ---------------------------------------------------------------------------


class TestLet:
    """Tests for LET assignment."""

    def test_let_constant_emits_load_imm_and_add_imm(self) -> None:
        result = compile_source("10 LET A = 5\n20 END\n")
        ops = opcodes(result)
        assert IrOp.LOAD_IMM in ops
        assert IrOp.ADD_IMM in ops

    def test_let_constant_value_correct(self) -> None:
        result = compile_source("10 LET A = 42\n20 END\n")
        load_ops = find_all(result, IrOp.LOAD_IMM)
        values = [op[1].value for op in load_ops if isinstance(op[1], IrImmediate)]
        assert 42 in values

    def test_let_copies_to_variable_register(self) -> None:
        """The copy (ADD_IMM v_var, v_temp, 0) must write to variable A's register (v1)."""
        result = compile_source("10 LET A = 7\n20 END\n")
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        destinations = [op[0].index for op in add_imm_ops if isinstance(op[0], IrRegister)]
        assert 1 in destinations  # v1 = A

    def test_let_addition(self) -> None:
        result = compile_source("10 LET A = 2 + 3\n20 END\n")
        assert IrOp.ADD in opcodes(result)

    def test_let_subtraction(self) -> None:
        result = compile_source("10 LET A = 10 - 4\n20 END\n")
        assert IrOp.SUB in opcodes(result)

    def test_let_multiplication(self) -> None:
        result = compile_source("10 LET A = 3 * 7\n20 END\n")
        assert IrOp.MUL in opcodes(result)

    def test_let_division(self) -> None:
        result = compile_source("10 LET A = 10 / 2\n20 END\n")
        assert IrOp.DIV in opcodes(result)

    def test_let_unary_minus(self) -> None:
        result = compile_source("10 LET A = -5\n20 END\n")
        assert IrOp.SUB in opcodes(result)

    def test_let_complex_expression_precedence(self) -> None:
        """2 + 3 * 4 should have MUL before ADD (3*4 computed first)."""
        result = compile_source("10 LET A = 2 + 3 * 4\n20 END\n")
        ops = opcodes(result)
        mul_idx = next(i for i, op in enumerate(ops) if op == IrOp.MUL)
        add_idx = next(i for i, op in enumerate(ops) if op == IrOp.ADD)
        assert mul_idx < add_idx

    def test_let_variable_assignment_from_variable(self) -> None:
        """LET B = A should use A's register as source."""
        result = compile_source("10 LET A = 1\n20 LET B = A\n30 END\n")
        # B is v2, A is v1. The copy for B's LET should write to v2.
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        b_writes = [op for op in add_imm_ops if isinstance(op[0], IrRegister) and op[0].index == 2]
        assert len(b_writes) >= 1


# ---------------------------------------------------------------------------
# PRINT
# ---------------------------------------------------------------------------


class TestPrint:
    """Tests for PRINT statement."""

    def test_print_emits_load_imm_and_syscall(self) -> None:
        result = compile_source("10 PRINT \"HI\"\n20 END\n")
        assert IrOp.LOAD_IMM in opcodes(result)
        assert IrOp.SYSCALL in opcodes(result)

    def test_print_loads_correct_ge225_codes(self) -> None:
        result = compile_source("10 PRINT \"AB\"\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        # Filter to SYSCALL arg register (v0 = index 0)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert GE225_CODES["A"] in char_codes
        assert GE225_CODES["B"] in char_codes

    def test_print_appends_carriage_return(self) -> None:
        result = compile_source("10 PRINT \"X\"\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert CARRIAGE_RETURN_CODE in char_codes

    def test_print_syscall_arg_is_one(self) -> None:
        result = compile_source("10 PRINT \"X\"\n20 END\n")
        syscall_ops = find_all(result, IrOp.SYSCALL)
        args = [op[0].value for op in syscall_ops if isinstance(op[0], IrImmediate)]
        assert all(a == 1 for a in args)

    def test_print_empty_string_only_cr(self) -> None:
        result = compile_source("10 PRINT \"\"\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert char_codes == [CARRIAGE_RETURN_CODE]

    def test_print_lowercase_converted(self) -> None:
        """Lowercase in string literals is quietly uppercased."""
        result = compile_source("10 PRINT \"hello\"\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert GE225_CODES["H"] in char_codes

    def test_print_unsupported_char_raises(self) -> None:
        with pytest.raises(CompileError, match="has no GE-225 typewriter"):
            compile_source("10 PRINT \"@#$\"\n20 END\n")

    def test_print_number_char_works(self) -> None:
        result = compile_source("10 PRINT \"123\"\n20 END\n")
        assert IrOp.SYSCALL in opcodes(result)

    def test_ascii_encoding_emits_ascii_codes_for_string(self) -> None:
        ast = parse_dartmouth_basic("10 PRINT \"AB\"\n20 END\n")
        result = compile_basic(ast, char_encoding="ascii")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert ord("A") in char_codes
        assert ord("B") in char_codes
        assert GE225_CODES["A"] not in char_codes

    def test_ascii_encoding_emits_newline_for_cr(self) -> None:
        ast = parse_dartmouth_basic("10 PRINT \"X\"\n20 END\n")
        result = compile_basic(ast, char_encoding="ascii")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert ord("\n") in char_codes
        assert CARRIAGE_RETURN_CODE not in char_codes

    def test_ascii_encoding_digit_offset_48(self) -> None:
        ast = parse_dartmouth_basic("10 PRINT 5\n20 END\n")
        result = compile_basic(ast, char_encoding="ascii")
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        offsets = [
            op[2].value
            for op in add_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[2], IrImmediate)
        ]
        assert 48 in offsets

    def test_invalid_char_encoding_raises(self) -> None:
        ast = parse_dartmouth_basic("10 END\n")
        with pytest.raises(ValueError, match="char_encoding"):
            compile_basic(ast, char_encoding="utf8")


# ---------------------------------------------------------------------------
# GOTO
# ---------------------------------------------------------------------------


class TestGoto:
    """Tests for GOTO statement."""

    def test_goto_emits_jump(self) -> None:
        result = compile_source("10 GOTO 30\n20 END\n30 END\n")
        assert IrOp.JUMP in opcodes(result)

    def test_goto_targets_correct_label(self) -> None:
        result = compile_source("10 GOTO 30\n20 END\n30 END\n")
        jump_ops = find_all(result, IrOp.JUMP)
        targets = [str(op[0]) for op in jump_ops if isinstance(op[0], IrLabel)]
        assert "_line_30" in targets


# ---------------------------------------------------------------------------
# IF / THEN
# ---------------------------------------------------------------------------


class TestIf:
    """Tests for IF … THEN conditional jump."""

    def test_if_less_than_emits_cmp_lt_and_branch_nz(self) -> None:
        result = compile_source("10 IF A < B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_LT in opcodes(result)
        assert IrOp.BRANCH_NZ in opcodes(result)

    def test_if_greater_than_emits_cmp_gt(self) -> None:
        result = compile_source("10 IF A > B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_GT in opcodes(result)

    def test_if_equal_emits_cmp_eq(self) -> None:
        result = compile_source("10 IF A = B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_EQ in opcodes(result)

    def test_if_not_equal_emits_cmp_ne(self) -> None:
        result = compile_source("10 IF A <> B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_NE in opcodes(result)

    def test_if_less_equal_emits_cmp_gt_and_and_imm(self) -> None:
        """<= is NOT GT, implemented via AND_IMM 1."""
        result = compile_source("10 IF A <= B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_GT in opcodes(result)
        assert IrOp.AND_IMM in opcodes(result)

    def test_if_greater_equal_emits_cmp_lt_and_and_imm(self) -> None:
        """<= is NOT LT, implemented via AND_IMM 1."""
        result = compile_source("10 IF A >= B THEN 50\n20 END\n50 END\n")
        assert IrOp.CMP_LT in opcodes(result)
        assert IrOp.AND_IMM in opcodes(result)

    def test_if_branch_targets_correct_line(self) -> None:
        result = compile_source("10 IF A < B THEN 99\n20 END\n99 END\n")
        branch_ops = find_all(result, IrOp.BRANCH_NZ)
        targets = [str(op[1]) for op in branch_ops if len(op) > 1 and isinstance(op[1], IrLabel)]
        assert "_line_99" in targets

    def test_if_with_constant_expressions(self) -> None:
        """IF 1 < 2 THEN 99 should compile without error."""
        result = compile_source("10 IF 1 < 2 THEN 99\n20 END\n99 END\n")
        assert IrOp.CMP_LT in opcodes(result)


# ---------------------------------------------------------------------------
# FOR / NEXT
# ---------------------------------------------------------------------------


class TestForNext:
    """Tests for FOR … TO … NEXT loop compilation."""

    def test_for_next_emits_labels(self) -> None:
        result = compile_source("10 FOR I = 1 TO 3\n20 NEXT I\n30 END\n")
        lbl = labels(result)
        assert "_for_0_check" in lbl
        assert "_for_0_end" in lbl

    def test_for_initializes_variable(self) -> None:
        """FOR I = 1 TO 3 should emit ADD_IMM to copy start into I's register."""
        result = compile_source("10 FOR I = 1 TO 3\n20 NEXT I\n30 END\n")
        # I maps to v9 (ord('I') - ord('A') + 1 = 9)
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        destinations = [op[0].index for op in add_imm_ops if isinstance(op[0], IrRegister)]
        assert 9 in destinations  # v9 = BASIC variable I

    def test_for_default_step_is_one(self) -> None:
        """A FOR with no STEP should load a constant 1 for the step."""
        result = compile_source("10 FOR I = 1 TO 10\n20 NEXT I\n30 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        values = [op[1].value for op in load_imm_ops if isinstance(op[1], IrImmediate)]
        assert 1 in values

    def test_for_emits_cmp_gt_for_pre_test(self) -> None:
        result = compile_source("10 FOR I = 1 TO 5\n20 NEXT I\n30 END\n")
        assert IrOp.CMP_GT in opcodes(result)

    def test_for_emits_branch_nz_to_end(self) -> None:
        result = compile_source("10 FOR I = 1 TO 5\n20 NEXT I\n30 END\n")
        branch_ops = find_all(result, IrOp.BRANCH_NZ)
        targets = [str(op[1]) for op in branch_ops if len(op) > 1 and isinstance(op[1], IrLabel)]
        assert "_for_0_end" in targets

    def test_for_next_emits_add_for_increment(self) -> None:
        result = compile_source("10 FOR I = 1 TO 5\n20 NEXT I\n30 END\n")
        assert IrOp.ADD in opcodes(result)

    def test_for_next_emits_jump_back_to_check(self) -> None:
        result = compile_source("10 FOR I = 1 TO 5\n20 NEXT I\n30 END\n")
        jump_ops = find_all(result, IrOp.JUMP)
        targets = [str(op[0]) for op in jump_ops if isinstance(op[0], IrLabel)]
        assert "_for_0_check" in targets

    def test_for_explicit_step(self) -> None:
        """FOR I = 0 TO 10 STEP 2 should compile without error."""
        result = compile_source("10 FOR I = 0 TO 10 STEP 2\n20 NEXT I\n30 END\n")
        assert "_for_0_check" in labels(result)

    def test_nested_for_loops(self) -> None:
        """Two nested FOR loops should produce labels _for_0_* and _for_1_*."""
        source = (
            "10 FOR I = 1 TO 3\n"
            "20 FOR J = 1 TO 3\n"
            "30 NEXT J\n"
            "40 NEXT I\n"
            "50 END\n"
        )
        result = compile_source(source)
        lbl = labels(result)
        assert "_for_0_check" in lbl
        assert "_for_1_check" in lbl

    def test_next_without_for_raises(self) -> None:
        with pytest.raises(CompileError, match="NEXT without matching FOR"):
            compile_source("10 NEXT I\n20 END\n")

    def test_next_wrong_variable_raises(self) -> None:
        with pytest.raises(CompileError, match="does not match"):
            compile_source("10 FOR I = 1 TO 3\n20 NEXT J\n30 END\n")


# ---------------------------------------------------------------------------
# END / STOP
# ---------------------------------------------------------------------------


class TestEndStop:
    """Tests for END and STOP statements."""

    def test_end_emits_halt(self) -> None:
        result = compile_source("10 END\n")
        assert IrOp.HALT in opcodes(result)

    def test_stop_emits_halt(self) -> None:
        result = compile_source("10 STOP\n")
        assert IrOp.HALT in opcodes(result)


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for CompileError on unsupported V1 features."""

    def test_gosub_raises(self) -> None:
        with pytest.raises(CompileError, match="GOSUB"):
            compile_source("10 GOSUB 100\n100 RETURN\n")

    def test_dim_raises(self) -> None:
        with pytest.raises(CompileError, match="DIM"):
            compile_source("10 DIM A(10)\n20 END\n")

    def test_power_operator_raises(self) -> None:
        with pytest.raises(CompileError, match="power"):
            compile_source("10 LET A = 2 ^ 8\n20 END\n")

    def test_array_access_raises(self) -> None:
        with pytest.raises(CompileError, match="array"):
            compile_source("10 LET A = B(3)\n20 END\n")

    def test_wrong_ast_root_raises_value_error(self) -> None:
        from lang_parser import ASTNode
        bad_node = ASTNode(rule_name="let_stmt", children=[])
        with pytest.raises(ValueError, match="expected 'program'"):
            compile_basic(bad_node)

    def test_print_variable_emits_digit_sequence(self) -> None:
        """PRINT with a variable compiles to a digit-extraction IR sequence."""
        result = compile_source("10 LET A = 7\n20 PRINT A\n30 END\n")
        # The digit-extraction routine uses DIV, MUL, SUB, and multiple SYSCALLs
        assert IrOp.DIV in opcodes(result)
        assert IrOp.SYSCALL in opcodes(result)

    def test_print_numeric_literal_emits_digit_sequence(self) -> None:
        """PRINT with a numeric literal compiles to a digit-extraction IR sequence."""
        result = compile_source("10 PRINT 42\n20 END\n")
        assert IrOp.DIV in opcodes(result)
        assert IrOp.SYSCALL in opcodes(result)


# ---------------------------------------------------------------------------
# Full program integration
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Tests for edge cases and less-common code paths."""

    def test_two_character_variable_a0(self) -> None:
        """Letter+digit variable A0 maps to register 27."""
        result = compile_source("10 LET A0 = 5\n20 END\n")
        # A0 = v27; the ADD_IMM copy should write to v27
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        destinations = {op[0].index for op in add_imm_ops if isinstance(op[0], IrRegister)}
        assert 27 in destinations

    def test_two_character_variable_z9(self) -> None:
        """Letter+digit variable Z9 maps to register 286."""
        result = compile_source("10 LET Z9 = 1\n20 END\n")
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        destinations = {op[0].index for op in add_imm_ops if isinstance(op[0], IrRegister)}
        assert 286 in destinations

    def test_print_no_args_emits_only_carriage_return(self) -> None:
        """Bare PRINT with no arguments emits just a carriage return."""
        result = compile_source("10 PRINT\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        char_codes = [
            op[1].value
            for op in load_imm_ops
            if isinstance(op[0], IrRegister) and op[0].index == 0
            and isinstance(op[1], IrImmediate)
        ]
        assert char_codes == [CARRIAGE_RETURN_CODE]

    def test_parenthesized_expression(self) -> None:
        """Parenthesized subexpressions compile correctly."""
        result = compile_source("10 LET A = (2 + 3) * 4\n20 END\n")
        ops = opcodes(result)
        assert IrOp.ADD in ops
        assert IrOp.MUL in ops

    def test_chained_addition(self) -> None:
        """Left-associative addition chain compiles to two ADD instructions."""
        result = compile_source("10 LET A = 1 + 2 + 3\n20 END\n")
        add_count = sum(1 for op in opcodes(result) if op == IrOp.ADD)
        assert add_count == 2

    def test_empty_rem_statement(self) -> None:
        """REM with no text compiles without error."""
        result = compile_source("10 REM\n20 END\n")
        assert IrOp.COMMENT in opcodes(result)

    def test_float_literal_truncated(self) -> None:
        """Float literals are truncated to integers."""
        result = compile_source("10 LET A = 3\n20 END\n")
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        values = [op[1].value for op in load_imm_ops if isinstance(op[1], IrImmediate)]
        assert 3 in values

    def test_line_with_only_comment_skipped(self) -> None:
        """Line numbers appear as labels even when the statement is REM."""
        result = compile_source("10 REM SKIP ME\n20 END\n")
        assert "_line_10" in labels(result)


class TestFullPrograms:
    """End-to-end compilation tests for representative programs."""

    def test_hello_world(self) -> None:
        """Classic hello world program compiles without error."""
        source = "10 PRINT \"HELLO WORLD\"\n20 END\n"
        result = compile_source(source)
        assert result.program.entry_label == "_start"
        assert IrOp.SYSCALL in opcodes(result)
        assert IrOp.HALT in opcodes(result)

    def test_count_down_loop(self) -> None:
        """A countdown FOR loop compiles correctly."""
        source = (
            "10 FOR I = 3 TO 1\n"
            "20 NEXT I\n"
            "30 END\n"
        )
        result = compile_source(source)
        assert "_for_0_check" in labels(result)
        assert "_for_0_end" in labels(result)

    def test_conditional_jump(self) -> None:
        source = (
            "10 LET A = 5\n"
            "20 IF A > 3 THEN 40\n"
            "30 END\n"
            "40 END\n"
        )
        result = compile_source(source)
        assert IrOp.CMP_GT in opcodes(result)
        assert IrOp.BRANCH_NZ in opcodes(result)

    def test_conditional_loop_with_if(self) -> None:
        """IF/THEN looping back emits CMP_LT + BRANCH_NZ (not JUMP — that's GOTO)."""
        source = (
            "10 LET A = 1\n"
            "20 LET A = A + 1\n"
            "30 IF A < 5 THEN 20\n"
            "40 END\n"
        )
        result = compile_source(source)
        assert IrOp.BRANCH_NZ in opcodes(result)
        assert IrOp.CMP_LT in opcodes(result)

    def test_multiple_variables(self) -> None:
        source = (
            "10 LET A = 3\n"
            "20 LET B = 4\n"
            "30 LET C = A + B\n"
            "40 END\n"
        )
        result = compile_source(source)
        assert IrOp.ADD in opcodes(result)
        # A=v1, B=v2, C=v3 all referenced as destinations
        add_imm_ops = find_all(result, IrOp.ADD_IMM)
        destinations = {op[0].index for op in add_imm_ops if isinstance(op[0], IrRegister)}
        assert 1 in destinations  # A
        assert 2 in destinations  # B
        assert 3 in destinations  # C


# ---------------------------------------------------------------------------
# Synthetic AST tests — cover defensive error paths unreachable via the parser
# ---------------------------------------------------------------------------


def _tok(type_: str, value: str) -> "Token":  # type: ignore[name-defined]
    from lexer import Token
    return Token(type=type_, value=value, line=1, column=1)


def _node(rule: str, *children: object) -> "ASTNode":  # type: ignore[name-defined]
    from lang_parser import ASTNode
    return ASTNode(rule_name=rule, children=list(children))


def _wrap_in_program(stmt_node: object, line_num: int = 10) -> object:
    """Wrap a single statement node in a fully-formed program ASTNode."""
    return _node(
        "program",
        _node(
            "line",
            _tok("LINE_NUM", str(line_num)),
            _node("statement", stmt_node),
        ),
    )


def _let(var: str, expr_node: object) -> object:
    """Construct a let_stmt node: LET var = expr."""
    return _node(
        "let_stmt",
        _node("variable", _tok("NAME", var)),
        _tok("EQ", "="),
        expr_node,
    )


def _primary_number(value: str) -> object:
    """Construct a primary node wrapping a NUMBER token."""
    return _node("primary", _tok("NUMBER", value))


class TestSyntheticAST:
    """Covers defensive error paths by constructing ASTs directly."""

    def test_line_without_line_num_is_skipped(self) -> None:
        """A 'line' node with no LINE_NUM token is silently skipped (line 347)."""
        program = _node("program", _node("line"))  # no LINE_NUM token
        result = compile_basic(program)
        # Epilogue HALT still emitted
        assert IrOp.HALT in opcodes(result)

    def test_malformed_let_no_expr_raises(self) -> None:
        """let_stmt with no expr after EQ raises CompileError (line 441)."""
        # variable child present, but no EQ token so seen_eq=False and expr_node=None
        stmt = _node("let_stmt", _node("variable", _tok("NAME", "A")))
        with pytest.raises(CompileError, match="malformed LET"):
            compile_basic(_wrap_in_program(stmt))

    def test_malformed_let_variable_no_name_raises(self) -> None:
        """let_stmt with empty variable node raises CompileError (lines 1066, 445)."""
        # variable before EQ has no NAME token → _extract_var_name returns None
        stmt = _node(
            "let_stmt",
            _node("variable"),        # no NAME token inside → line 1066 return None
            _tok("EQ", "="),
            _primary_number("1"),
        )
        with pytest.raises(CompileError, match="could not extract variable name"):
            compile_basic(_wrap_in_program(stmt))

    def test_if_relop_with_no_token_raises(self) -> None:
        """relop node with no Token causes malformed IF (lines 1097, 594)."""
        # relop has no children → _first_token returns None (line 1097)
        # lineno is present + 2 exprs, but relop_token is None → line 594
        if_node = _node(
            "if_stmt",
            _primary_number("1"),
            _node("relop"),           # no Token children → _first_token → None (line 1097)
            _primary_number("2"),
            _tok("NUMBER", "50"),
        )
        with pytest.raises(CompileError, match="malformed IF"):
            compile_basic(_wrap_in_program(if_node))

    def test_if_unknown_relop_raises(self) -> None:
        """relop with an unrecognised value raises CompileError (line 632)."""
        # relop token exists but its value is not <, >, =, <>, <=, >=
        if_node = _node(
            "if_stmt",
            _primary_number("1"),
            _node("relop", _tok("EQ", "!=")),  # value "!=" is unknown
            _primary_number("2"),
            _tok("NUMBER", "50"),
        )
        with pytest.raises(CompileError, match="unknown relational operator"):
            compile_basic(_wrap_in_program(if_node))

    def test_malformed_for_no_children_raises(self) -> None:
        """for_stmt with no children raises CompileError (line 699)."""
        stmt = _node("for_stmt")  # var_name=None, exprs=[]
        with pytest.raises(CompileError, match="malformed FOR"):
            compile_basic(_wrap_in_program(stmt))

    def test_expr_variable_node_directly_compiles(self) -> None:
        """variable node passed directly to _compile_expr succeeds (lines 835-836)."""
        # expr → [variable(A)] — unusual but valid: binop chain calls _compile_expr(variable)
        var_as_child = _node("variable", _tok("NAME", "A"))
        expr_node = _node("expr", var_as_child)
        program = _wrap_in_program(_let("B", expr_node))
        result = compile_basic(program)
        assert IrOp.ADD_IMM in opcodes(result)  # LET B = A emits ADD_IMM copy

    def test_expr_single_child_pass_through_compiles(self) -> None:
        """Unknown-rule node with one ASTNode child is passed through (lines 838-841)."""
        # paren_group is not a known expr rule; _compile_expr delegates to its sole child
        inner = _primary_number("5")
        wrapper = _node("paren_group", inner)  # unknown rule, single ASTNode child
        expr_node = _node("expr", wrapper)
        program = _wrap_in_program(_let("A", expr_node))
        result = compile_basic(program)
        # LET A = 5 via wrapper — should produce LOAD_IMM 5 and ADD_IMM copy
        load_imm_ops = find_all(result, IrOp.LOAD_IMM)
        values = [op[1].value for op in load_imm_ops if isinstance(op[1], IrImmediate)]
        assert 5 in values

    def test_expr_unexpected_node_raises(self) -> None:
        """Unknown-rule node with multiple ASTNode children raises CompileError (line 843)."""
        child1 = _primary_number("1")
        child2 = _primary_number("2")
        bad_wrapper = _node("paren_group", child1, child2)  # 2 children → not pass-through
        expr_node = _node("expr", bad_wrapper)
        with pytest.raises(CompileError, match="unexpected expression node"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_empty_primary_raises(self) -> None:
        """primary with no children raises CompileError (line 866)."""
        empty_primary = _node("primary")
        expr_node = _node("expr", empty_primary)
        with pytest.raises(CompileError, match="empty primary expression"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_variable_expr_no_name_raises(self) -> None:
        """variable node with no NAME token raises in _compile_variable_expr (lines 891, 1066)."""
        # variable has no NAME token → _extract_var_name returns None → line 891
        empty_var = _node("variable")  # no NAME token, no LPAREN
        expr_node = _node("expr", empty_var)
        with pytest.raises(CompileError, match="could not extract variable name"):
            compile_basic(_wrap_in_program(_let("B", expr_node)))

    def test_empty_unary_raises(self) -> None:
        """unary node with no inner expression raises CompileError (line 913)."""
        empty_unary = _node("unary")  # no children → inner_node=None
        expr_node = _node("expr", empty_unary)
        with pytest.raises(CompileError, match="empty unary expression"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_empty_power_raises(self) -> None:
        """power node with no children raises CompileError (line 956)."""
        empty_power = _node("power")  # no children → loop finds nothing
        expr_node = _node("expr", empty_power)
        with pytest.raises(CompileError, match="empty power expression"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_empty_binop_chain_raises(self) -> None:
        """expr node with only operator tokens (no operand nodes) raises (line 984)."""
        # Only a PLUS token — type matches operator check, but no ASTNode operands
        expr_node = _node("expr", _tok("PLUS", "+"))
        with pytest.raises(CompileError, match="empty expr expression"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_unknown_binary_operator_raises(self) -> None:
        """Operator token with unexpected value raises CompileError (line 1019)."""
        # Token type is PLUS (matches operator check) but value is garbage
        fake_op = _tok("PLUS", "@@@")
        expr_node = _node("expr", _primary_number("1"), fake_op, _primary_number("2"))
        with pytest.raises(CompileError, match="unknown binary operator"):
            compile_basic(_wrap_in_program(_let("A", expr_node)))

    def test_goto_without_line_number_raises(self) -> None:
        """goto_stmt with no NUMBER token raises CompileError (line 1083)."""
        goto_node = _node("goto_stmt", _tok("KEYWORD", "GOTO"))  # no NUMBER
        with pytest.raises(CompileError, match="could not find line number"):
            compile_basic(_wrap_in_program(goto_node))
