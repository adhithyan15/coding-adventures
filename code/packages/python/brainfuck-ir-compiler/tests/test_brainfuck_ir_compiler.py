"""Tests for the brainfuck_ir_compiler package.

Tests cover:
- BuildConfig: defaults, debug_config(), release_config()
- Empty program: prologue + HALT, tape data decl, version, entry label
- Single commands: +, -, >, <, ., ,
- MaskByteArithmetic=False: no AND_IMM emitted
- Bounds checking: CMP_GT/CMP_LT, BRANCH_NZ, __trap_oob label
- No bounds checks in release mode
- Loop compilation: labels, BRANCH_Z, JUMP
- Nested loops
- Source map: SourceToAst entries, AstToIr entries
- IR printer integration (roundtrip)
- Error cases: wrong AST root, zero/negative tape size
- Instruction ID uniqueness
- Complex programs: Hello World fragment, cat program
"""

from __future__ import annotations

import pytest

from brainfuck import parse_brainfuck
from brainfuck_ir_compiler import (
    BuildConfig,
    CompileResult,
    compile_brainfuck,
    debug_config,
    release_config,
)
from compiler_ir import IrOp, print_ir, parse_ir


# =============================================================================
# Test helpers
# =============================================================================


def compile_source(source: str, config: BuildConfig) -> CompileResult:
    """Parse and compile a Brainfuck source string."""
    ast = parse_brainfuck(source)
    return compile_brainfuck(ast, "test.bf", config)


def count_opcode(result: CompileResult, opcode: IrOp) -> int:
    """Count how many instructions with the given opcode appear."""
    return sum(1 for instr in result.program.instructions if instr.opcode == opcode)


def has_label(result: CompileResult, name: str) -> bool:
    """Check if the program contains a LABEL instruction with the given name."""
    from compiler_ir import IrLabel
    for instr in result.program.instructions:
        if instr.opcode == IrOp.LABEL and instr.operands:
            if isinstance(instr.operands[0], IrLabel) and instr.operands[0].name == name:
                return True
    return False


# =============================================================================
# BuildConfig tests
# =============================================================================


class TestBuildConfig:
    """Tests for BuildConfig and factory functions."""

    def test_debug_config_bounds_checks(self) -> None:
        cfg = debug_config()
        assert cfg.insert_bounds_checks is True

    def test_debug_config_debug_locs(self) -> None:
        cfg = debug_config()
        assert cfg.insert_debug_locs is True

    def test_debug_config_mask_byte_arithmetic(self) -> None:
        cfg = debug_config()
        assert cfg.mask_byte_arithmetic is True

    def test_debug_config_tape_size(self) -> None:
        cfg = debug_config()
        assert cfg.tape_size == 30000

    def test_release_config_no_bounds_checks(self) -> None:
        cfg = release_config()
        assert cfg.insert_bounds_checks is False

    def test_release_config_no_debug_locs(self) -> None:
        cfg = release_config()
        assert cfg.insert_debug_locs is False

    def test_release_config_mask_byte_arithmetic(self) -> None:
        cfg = release_config()
        assert cfg.mask_byte_arithmetic is True

    def test_release_config_tape_size(self) -> None:
        cfg = release_config()
        assert cfg.tape_size == 30000

    def test_custom_config(self) -> None:
        cfg = BuildConfig(
            insert_bounds_checks=True,
            insert_debug_locs=False,
            mask_byte_arithmetic=False,
            tape_size=1000,
        )
        assert cfg.tape_size == 1000
        assert cfg.mask_byte_arithmetic is False


# =============================================================================
# Empty program
# =============================================================================


class TestEmptyProgram:
    """Tests for compiling an empty Brainfuck program."""

    def test_has_start_label(self) -> None:
        result = compile_source("", release_config())
        assert has_label(result, "_start")

    def test_has_exactly_one_halt(self) -> None:
        result = compile_source("", release_config())
        assert count_opcode(result, IrOp.HALT) == 1

    def test_version_is_1(self) -> None:
        result = compile_source("", release_config())
        assert result.program.version == 1

    def test_entry_label_is_start(self) -> None:
        result = compile_source("", release_config())
        assert result.program.entry_label == "_start"

    def test_has_tape_data_decl(self) -> None:
        result = compile_source("", release_config())
        assert len(result.program.data) == 1
        assert result.program.data[0].label == "tape"

    def test_tape_size_is_30000(self) -> None:
        result = compile_source("", release_config())
        assert result.program.data[0].size == 30000

    def test_tape_init_is_zero(self) -> None:
        result = compile_source("", release_config())
        assert result.program.data[0].init == 0

    def test_has_load_addr_prologue(self) -> None:
        result = compile_source("", release_config())
        assert count_opcode(result, IrOp.LOAD_ADDR) >= 1

    def test_has_load_imm_prologue(self) -> None:
        """Prologue emits LOAD_IMM v1, 0 for the tape pointer."""
        result = compile_source("", release_config())
        assert count_opcode(result, IrOp.LOAD_IMM) >= 1


# =============================================================================
# Single command compilation
# =============================================================================


class TestIncrementCommand:
    """Tests for the '+' (INC) command."""

    def test_has_load_byte(self) -> None:
        result = compile_source("+", release_config())
        assert count_opcode(result, IrOp.LOAD_BYTE) >= 1

    def test_has_store_byte(self) -> None:
        result = compile_source("+", release_config())
        assert count_opcode(result, IrOp.STORE_BYTE) >= 1

    def test_has_and_imm_for_masking(self) -> None:
        result = compile_source("+", release_config())
        assert count_opcode(result, IrOp.AND_IMM) >= 1

    def test_add_imm_delta_is_1(self) -> None:
        """INC should use ADD_IMM with delta +1."""
        from compiler_ir import IrImmediate
        result = compile_source("+", release_config())
        found = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.ADD_IMM and len(instr.operands) >= 3:
                if isinstance(instr.operands[2], IrImmediate) and instr.operands[2].value == 1:
                    found = True
                    break
        assert found, "Expected ADD_IMM with delta +1 for INC"

    def test_no_and_imm_when_masking_off(self) -> None:
        cfg = release_config()
        cfg.mask_byte_arithmetic = False
        result = compile_source("+", cfg)
        # No AND_IMM should appear in the instruction stream for INC
        assert count_opcode(result, IrOp.AND_IMM) == 0


class TestDecrementCommand:
    """Tests for the '-' (DEC) command."""

    def test_add_imm_delta_is_minus_1(self) -> None:
        """DEC should use ADD_IMM with delta -1."""
        from compiler_ir import IrImmediate
        result = compile_source("-", release_config())
        found = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.ADD_IMM and len(instr.operands) >= 3:
                if isinstance(instr.operands[2], IrImmediate) and instr.operands[2].value == -1:
                    found = True
                    break
        assert found, "Expected ADD_IMM with delta -1 for DEC"


class TestRightCommand:
    """Tests for the '>' (RIGHT) command."""

    def test_add_imm_v1_by_1(self) -> None:
        """RIGHT should emit ADD_IMM v1, v1, 1."""
        from compiler_ir import IrImmediate, IrRegister
        result = compile_source(">", release_config())
        found = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.ADD_IMM and len(instr.operands) >= 3:
                dst = instr.operands[0]
                imm = instr.operands[2]
                if (isinstance(dst, IrRegister) and dst.index == 1
                        and isinstance(imm, IrImmediate) and imm.value == 1):
                    found = True
                    break
        assert found, "Expected ADD_IMM v1, v1, 1 for RIGHT"


class TestLeftCommand:
    """Tests for the '<' (LEFT) command."""

    def test_add_imm_v1_by_minus_1(self) -> None:
        """LEFT should emit ADD_IMM v1, v1, -1."""
        from compiler_ir import IrImmediate, IrRegister
        result = compile_source("<", release_config())
        found = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.ADD_IMM and len(instr.operands) >= 3:
                dst = instr.operands[0]
                imm = instr.operands[2]
                if (isinstance(dst, IrRegister) and dst.index == 1
                        and isinstance(imm, IrImmediate) and imm.value == -1):
                    found = True
                    break
        assert found, "Expected ADD_IMM v1, v1, -1 for LEFT"


class TestOutputCommand:
    """Tests for the '.' (OUTPUT) command."""

    def test_has_syscall(self) -> None:
        result = compile_source(".", release_config())
        assert count_opcode(result, IrOp.SYSCALL) >= 1

    def test_syscall_number_is_1(self) -> None:
        """OUTPUT uses SYSCALL 1 (write)."""
        from compiler_ir import IrImmediate
        result = compile_source(".", release_config())
        for instr in result.program.instructions:
            if instr.opcode == IrOp.SYSCALL and instr.operands:
                if isinstance(instr.operands[0], IrImmediate) and instr.operands[0].value == 1:
                    return
        pytest.fail("Expected SYSCALL 1 (write) for OUTPUT")

    def test_output_copies_value_with_add_imm_zero(self) -> None:
        from compiler_ir import IrImmediate, IrRegister

        result = compile_source(".", release_config())
        for instr in result.program.instructions:
            if instr.opcode != IrOp.ADD_IMM or len(instr.operands) < 3:
                continue
            dst = instr.operands[0]
            src = instr.operands[1]
            imm = instr.operands[2]
            if (
                isinstance(dst, IrRegister)
                and dst.index == 4
                and isinstance(src, IrRegister)
                and src.index == 2
                and isinstance(imm, IrImmediate)
                and imm.value == 0
            ):
                return
        pytest.fail("Expected ADD_IMM v4, v2, 0 for OUTPUT")


class TestInputCommand:
    """Tests for the ',' (INPUT) command."""

    def test_has_syscall_2(self) -> None:
        """INPUT uses SYSCALL 2 (read)."""
        from compiler_ir import IrImmediate
        result = compile_source(",", release_config())
        for instr in result.program.instructions:
            if instr.opcode == IrOp.SYSCALL and instr.operands:
                if isinstance(instr.operands[0], IrImmediate) and instr.operands[0].value == 2:
                    return
        pytest.fail("Expected SYSCALL 2 (read) for INPUT")

    def test_has_store_byte(self) -> None:
        result = compile_source(",", release_config())
        assert count_opcode(result, IrOp.STORE_BYTE) >= 1


# =============================================================================
# Bounds checking
# =============================================================================


class TestBoundsChecking:
    """Tests for debug-mode bounds checks."""

    def test_right_has_cmp_gt(self) -> None:
        result = compile_source(">", debug_config())
        assert count_opcode(result, IrOp.CMP_GT) >= 1

    def test_right_has_branch_nz(self) -> None:
        result = compile_source(">", debug_config())
        assert count_opcode(result, IrOp.BRANCH_NZ) >= 1

    def test_right_has_trap_label(self) -> None:
        result = compile_source(">", debug_config())
        assert has_label(result, "__trap_oob")

    def test_left_has_cmp_lt(self) -> None:
        result = compile_source("<", debug_config())
        assert count_opcode(result, IrOp.CMP_LT) >= 1

    def test_release_no_cmp_gt(self) -> None:
        result = compile_source("><", release_config())
        assert count_opcode(result, IrOp.CMP_GT) == 0

    def test_release_no_cmp_lt(self) -> None:
        result = compile_source("><", release_config())
        assert count_opcode(result, IrOp.CMP_LT) == 0

    def test_release_no_trap_label(self) -> None:
        result = compile_source("><", release_config())
        assert not has_label(result, "__trap_oob")

    def test_debug_has_load_imm_for_max_ptr(self) -> None:
        """Debug prologue adds LOAD_IMM for v5 (max pointer) and v6 (zero)."""
        result = compile_source("", debug_config())
        # Release emits 2 LOAD_IMMs (v0 gets LOAD_ADDR, v1 gets LOAD_IMM 0).
        # Debug emits 2 more (v5, v6).
        assert count_opcode(result, IrOp.LOAD_IMM) >= 3


# =============================================================================
# Loop compilation
# =============================================================================


class TestLoopCompilation:
    """Tests for loop [body] compilation."""

    def test_simple_loop_start_label(self) -> None:
        result = compile_source("[-]", release_config())
        assert has_label(result, "loop_0_start")

    def test_simple_loop_end_label(self) -> None:
        result = compile_source("[-]", release_config())
        assert has_label(result, "loop_0_end")

    def test_simple_loop_has_branch_z(self) -> None:
        result = compile_source("[-]", release_config())
        assert count_opcode(result, IrOp.BRANCH_Z) >= 1

    def test_simple_loop_has_jump(self) -> None:
        result = compile_source("[-]", release_config())
        assert count_opcode(result, IrOp.JUMP) >= 1

    def test_empty_loop(self) -> None:
        result = compile_source("[]", release_config())
        assert has_label(result, "loop_0_start")
        assert has_label(result, "loop_0_end")
        assert count_opcode(result, IrOp.BRANCH_Z) >= 1

    def test_nested_loops(self) -> None:
        result = compile_source("[>[+<-]]", release_config())
        assert has_label(result, "loop_0_start")
        assert has_label(result, "loop_1_start")

    def test_multiple_top_level_loops(self) -> None:
        result = compile_source("[+][-]", release_config())
        assert has_label(result, "loop_0_start")
        assert has_label(result, "loop_1_start")
        assert has_label(result, "loop_0_end")
        assert has_label(result, "loop_1_end")


# =============================================================================
# Source map tests
# =============================================================================


class TestSourceMap:
    """Tests for source map generation."""

    def test_plus_dot_two_source_entries(self) -> None:
        result = compile_source("+.", release_config())
        assert len(result.source_map.source_to_ast.entries) == 2

    def test_plus_source_at_column_1(self) -> None:
        result = compile_source("+.", release_config())
        entry = result.source_map.source_to_ast.entries[0]
        assert entry.pos.column == 1

    def test_dot_source_at_column_2(self) -> None:
        result = compile_source("+.", release_config())
        entry = result.source_map.source_to_ast.entries[1]
        assert entry.pos.column == 2

    def test_filename_in_source_entries(self) -> None:
        result = compile_source("+.", release_config())
        for entry in result.source_map.source_to_ast.entries:
            assert entry.pos.file == "test.bf"

    def test_inc_has_four_ir_ids(self) -> None:
        """'+' with masking on emits 4 instructions: LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE."""
        result = compile_source("+", release_config())
        assert len(result.source_map.ast_to_ir.entries) == 1
        assert len(result.source_map.ast_to_ir.entries[0].ir_ids) == 4

    def test_inc_has_three_ir_ids_when_no_mask(self) -> None:
        """'+' without masking emits 3 instructions: LOAD_BYTE, ADD_IMM, STORE_BYTE."""
        cfg = release_config()
        cfg.mask_byte_arithmetic = False
        result = compile_source("+", cfg)
        assert len(result.source_map.ast_to_ir.entries) == 1
        assert len(result.source_map.ast_to_ir.entries[0].ir_ids) == 3

    def test_loop_has_source_entry(self) -> None:
        """A loop construct should appear in SourceToAst."""
        result = compile_source("[-]", release_config())
        # loop + command '-' = 2 SourceToAst entries
        assert len(result.source_map.source_to_ast.entries) >= 2

    def test_source_map_not_none(self) -> None:
        result = compile_source("+", release_config())
        assert result.source_map is not None
        assert result.source_map.source_to_ast is not None
        assert result.source_map.ast_to_ir is not None


# =============================================================================
# IR printer integration
# =============================================================================


class TestIRPrinterIntegration:
    """Tests that compiled IR can be printed and parsed back."""

    def test_printed_ir_has_version(self) -> None:
        result = compile_source("+.", release_config())
        text = print_ir(result.program)
        assert ".version 1" in text

    def test_printed_ir_has_data(self) -> None:
        result = compile_source("+.", release_config())
        text = print_ir(result.program)
        assert ".data tape 30000 0" in text

    def test_printed_ir_has_entry(self) -> None:
        result = compile_source("+.", release_config())
        text = print_ir(result.program)
        assert ".entry _start" in text

    def test_printed_ir_has_load_byte(self) -> None:
        result = compile_source("+.", release_config())
        text = print_ir(result.program)
        assert "LOAD_BYTE" in text

    def test_printed_ir_has_halt(self) -> None:
        result = compile_source("+.", release_config())
        text = print_ir(result.program)
        assert "HALT" in text

    def test_roundtrip(self) -> None:
        """print_ir → parse_ir should produce the same instruction count."""
        result = compile_source("++[-].", release_config())
        text = print_ir(result.program)
        parsed = parse_ir(text)
        assert len(parsed.instructions) == len(result.program.instructions)


# =============================================================================
# Instruction ID uniqueness
# =============================================================================


class TestInstructionIDs:
    """Tests that instruction IDs are unique across the program."""

    def test_ids_are_unique(self) -> None:
        result = compile_source("++[>+<-].", release_config())
        seen: set[int] = set()
        for instr in result.program.instructions:
            if instr.id == -1:
                continue  # labels have ID -1, which is fine
            assert instr.id not in seen, f"Duplicate instruction ID: {instr.id}"
            seen.add(instr.id)

    def test_ids_start_at_zero(self) -> None:
        result = compile_source("+", release_config())
        non_label_ids = [i.id for i in result.program.instructions if i.id != -1]
        assert 0 in non_label_ids


# =============================================================================
# Error cases
# =============================================================================


class TestErrorCases:
    """Tests for compiler error conditions."""

    def test_wrong_ast_root_raises(self) -> None:
        from lang_parser import ASTNode
        bad_ast = ASTNode(rule_name="not_a_program", children=[])
        with pytest.raises(ValueError, match="expected 'program'"):
            compile_brainfuck(bad_ast, "test.bf", release_config())

    def test_zero_tape_size_raises(self) -> None:
        ast = parse_brainfuck("")
        cfg = release_config()
        cfg.tape_size = 0
        with pytest.raises(ValueError, match="tape_size"):
            compile_brainfuck(ast, "test.bf", cfg)

    def test_negative_tape_size_raises(self) -> None:
        ast = parse_brainfuck("")
        cfg = release_config()
        cfg.tape_size = -1
        with pytest.raises(ValueError, match="tape_size"):
            compile_brainfuck(ast, "test.bf", cfg)


# =============================================================================
# Complex programs
# =============================================================================


class TestComplexPrograms:
    """Tests for more complex Brainfuck programs."""

    def test_hello_world_fragment(self) -> None:
        """Set cell 0 to 72 ('H') and output: ++++++++ [>+++++++++<-] >."""
        source = "++++++++[>+++++++++<-]>."
        result = compile_source(source, release_config())

        # Should have a loop
        assert has_label(result, "loop_0_start")

        # Should have output syscall
        from compiler_ir import IrImmediate
        found_output = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.SYSCALL and instr.operands:
                if isinstance(instr.operands[0], IrImmediate) and instr.operands[0].value == 1:
                    found_output = True
                    break
        assert found_output, "Expected SYSCALL 1 (write) for output"

    def test_cat_program(self) -> None:
        """Cat program: ,[.,] — read and echo until zero."""
        from compiler_ir import IrImmediate
        result = compile_source(",[.,]", release_config())

        found_read = False
        found_write = False
        for instr in result.program.instructions:
            if instr.opcode == IrOp.SYSCALL and instr.operands:
                if isinstance(instr.operands[0], IrImmediate):
                    if instr.operands[0].value == 2:
                        found_read = True
                    if instr.operands[0].value == 1:
                        found_write = True

        assert found_read, "Expected SYSCALL 2 (read) in cat program"
        assert found_write, "Expected SYSCALL 1 (write) in cat program"

    def test_custom_tape_size(self) -> None:
        cfg = release_config()
        cfg.tape_size = 1000
        result = compile_source("", cfg)
        assert result.program.data[0].size == 1000

    def test_all_eight_commands(self) -> None:
        """All 8 Brainfuck commands can be compiled without error."""
        source = "><+-.,[]"
        result = compile_source(source, release_config())
        # Should have HALT at the end
        assert count_opcode(result, IrOp.HALT) == 1

    def test_many_commands_produce_unique_ids(self) -> None:
        """A longer program still has unique instruction IDs."""
        source = "++++++++[>+++++++++<-]>."
        result = compile_source(source, release_config())
        seen: set[int] = set()
        for instr in result.program.instructions:
            if instr.id != -1:
                assert instr.id not in seen
                seen.add(instr.id)
