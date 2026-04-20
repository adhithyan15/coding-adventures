"""Tests for ALGOL 60 to compiler-IR lowering."""

import pytest
from algol_parser import parse_algol
from algol_type_checker import FRAME_WORD_SIZE, FrameSlot, check_algol
from compiler_ir import IrOp
from lang_parser import ASTNode

from algol_ir_compiler import CompileError, __version__, compile_algol


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestAlgolIrCompiler:
    """Small programs lower to the IR shapes expected by the WASM backend."""

    def test_compiles_literal_result(self) -> None:
        result = compile_algol(parse_algol("begin integer result; result := 7 end"))
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert opcodes[0] == IrOp.LABEL
        assert IrOp.LOAD_IMM in opcodes
        assert IrOp.LOAD_ADDR in opcodes
        assert IrOp.STORE_WORD in opcodes
        assert opcodes[-1] == IrOp.HALT
        assert result.variable_registers["result"] == 20
        assert result.variable_slots["result@block0"] == 20
        assert result.frame_offsets[0] == 0
        assert result.frame_memory_label == "__algol_frames"
        assert result.program.data[0].label == "__algol_frames"

    def test_compiles_arithmetic_precedence(self) -> None:
        result = compile_algol(
            parse_algol("begin integer result; result := 1 + 2 * 3 end")
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert opcodes.index(IrOp.MUL) < opcodes.index(IrOp.ADD)

    def test_compiles_structured_if_labels(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; if 1 < 2 then result := 7 else result := 8 end"
            )
        )
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]
        assert "if_0_else" in labels
        assert "if_0_end" in labels

    def test_compiles_for_loop_labels(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result, i; "
                "for i := 1 step 1 until 3 do result := result + i "
                "end"
            )
        )
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]
        assert "loop_0_start" in labels
        assert "loop_0_end" in labels

    def test_compiles_unary_minus_and_mod(self) -> None:
        result = compile_algol(
            parse_algol("begin integer result; result := -10 mod 4 end")
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert IrOp.DIV in opcodes
        assert IrOp.MUL in opcodes
        assert opcodes.count(IrOp.SUB) >= 2

    def test_compiles_boolean_not_or_and_comparison_forms(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "if not false or (1 <= 2 and 3 >= 3) "
                "then result := 1 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert IrOp.CMP_GT in opcodes
        assert IrOp.AND in opcodes

    def test_compiles_greater_equal_via_less_than_inversion(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; if 3 >= 3 then result := 1 else result := 0 end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert IrOp.CMP_LT in opcodes
        assert IrOp.AND_IMM in opcodes

    def test_compiles_not_equal_comparison(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; if 1 != 2 then result := 1 else result := 0 end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert IrOp.CMP_NE in opcodes

    def test_compiles_nested_block(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "begin integer inner; inner := 4; result := inner end "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert IrOp.LOAD_WORD in opcodes
        assert IrOp.STORE_WORD in opcodes
        assert result.variable_slots["result@block0"] == 20
        assert result.variable_slots["inner@block1"] == 20
        assert result.frame_offsets[1] > result.frame_offsets[0]

    def test_inner_block_writes_outer_slot_through_static_link(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "begin integer inner; result := 3; inner := result end "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        assert opcodes.count(IrOp.LOAD_WORD) >= 3
        assert opcodes.count(IrOp.STORE_WORD) >= 8

    def test_shadowed_variables_keep_separate_frame_slots(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer x, result; "
                "x := 1; "
                "begin integer x; x := 9; result := x end "
                "end"
            )
        )
        assert result.variable_slots["x@block0"] == 20
        assert result.variable_slots["x@block1"] == 20
        assert result.variable_slots["result@block0"] == 24

    def test_rejects_oversized_frame_memory_before_wasm_allocation(self) -> None:
        typed = check_algol(parse_algol("begin integer result; result := 1 end"))
        assert typed.semantic is not None
        assert typed.semantic.root_block is not None

        layout = typed.semantic.root_block.frame_layout
        for index in range(17000):
            layout.slots.append(
                FrameSlot(
                    symbol_id=1000 + index,
                    name=f"synthetic_{index}",
                    type_name="integer",
                    offset=layout.header_size + (len(layout.slots) * FRAME_WORD_SIZE),
                    size=FRAME_WORD_SIZE,
                )
            )

        with pytest.raises(CompileError, match="phase-3 limit"):
            compile_algol(typed)

    def test_compiles_integer_value_procedure_call(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "integer procedure inc(x); value x; integer x; "
                "begin inc := x + 1 end; "
                "result := inc(4) "
                "end"
            )
        )
        calls = [
            instruction
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        labels = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.LABEL
        ]
        assert calls[0].operands[0].name.startswith("_fn_algol_")
        assert len(calls[0].operands) == 3
        assert any(label.startswith("_fn_algol_") for label in labels)
        assert result.procedure_signatures[calls[0].operands[0].name] == 2

    def test_compiles_recursive_procedure_call(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "integer procedure fact(n); value n; integer n; "
                "begin if n = 0 then fact := 1 else fact := n * fact(n - 1) end; "
                "result := fact(4) "
                "end"
            )
        )
        call_labels = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        assert len(call_labels) == 2
        assert call_labels[0] == call_labels[1]

    def test_raises_for_type_error_input(self) -> None:
        with pytest.raises(CompileError):
            compile_algol(parse_algol("begin integer result; result := false end"))

    def test_raises_for_missing_block(self) -> None:
        with pytest.raises(CompileError, match="must contain a block"):
            compile_algol(ASTNode("program", []))

    def test_raises_for_missing_result_variable(self) -> None:
        with pytest.raises(CompileError, match="result"):
            compile_algol(parse_algol("begin integer x; x := 1 end"))

    def test_raises_for_unsupported_exponentiation(self) -> None:
        with pytest.raises(CompileError):
            compile_algol(parse_algol("begin integer result; result := 2 ** 3 end"))
