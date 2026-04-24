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

    def test_compiles_local_goto_to_algol_label(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result, i; "
                "loop: i := i + 1; "
                "if i < 3 then goto loop; "
                "result := i "
                "end"
            )
        )
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]
        jumps = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.JUMP
        ]

        algol_labels = [label for label in labels if label.startswith("algol_label_")]
        assert len(algol_labels) == 1
        assert algol_labels[0] in jumps

    def test_compiles_direct_nonlocal_block_goto_with_frame_unwind(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "begin integer inner; goto done; inner := 99 end; "
                "done: result := 7 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]
        jumps = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.JUMP
        ]

        target = next(label for label in labels if label.startswith("algol_label_"))
        assert target in jumps
        assert opcodes.count(IrOp.LOAD_WORD) >= 1
        assert opcodes.count(IrOp.STORE_WORD) >= 1

    def test_compiles_procedure_crossing_goto_with_pending_transfer(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "procedure escape; begin goto done end; "
                "escape; "
                "done: result := 7 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]

        assert IrOp.CALL in opcodes
        assert opcodes.count(IrOp.RET) >= 2
        assert any(label.startswith("algol_label_") for label in labels)

    def test_compiles_conditional_designational_goto(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "goto if true then left else right; "
                "left: result := 1; goto done; "
                "right: result := 2; "
                "done: "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]

        assert IrOp.BRANCH_Z in opcodes
        assert any(label.startswith("algol_label_") for label in labels)

    def test_compiles_switch_designational_goto(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result, i; "
                "switch s := first, second; "
                "i := 2; goto s[i]; "
                "first: result := 1; goto done; "
                "second: result := 2; "
                "done: "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]

        assert IrOp.CMP_EQ in opcodes
        assert any(label.startswith("switch_0_1_next") for label in labels)

    def test_repeated_switch_selections_get_distinct_dispatch_labels(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result, i; "
                "switch s := first, second; "
                "i := 1; goto s[i]; "
                "first: i := 2; goto s[i]; "
                "second: result := 7 "
                "end"
            )
        )
        labels = [
            instr.operands[0].name
            for instr in result.program.instructions
            if instr.opcode == IrOp.LABEL
        ]

        assert any(label.startswith("switch_0_1_next") for label in labels)
        assert any(label.startswith("switch_1_1_next") for label in labels)

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

    def test_compiles_boolean_variable_storage_and_readback(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "boolean flag; "
                "flag := true; "
                "if flag then result := 1 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert opcodes.count(IrOp.STORE_WORD) >= 2
        assert opcodes.count(IrOp.LOAD_WORD) >= 2

    def test_compiles_real_variable_storage_and_comparison(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "real x; "
                "x := 1.5; "
                "if x > 1.0 then result := 7 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert IrOp.LOAD_F64_IMM in opcodes
        assert IrOp.STORE_F64 in opcodes
        assert IrOp.LOAD_F64 in opcodes
        assert IrOp.F64_CMP_GT in opcodes

    def test_compiles_integer_to_real_promotion(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "real x; "
                "x := 1; "
                "if x = 1.0 then result := 9 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert IrOp.F64_FROM_I32 in opcodes

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
        for index in range(16375):
            layout.slots.append(
                FrameSlot(
                    symbol_id=1000 + index,
                    name=f"synthetic_{index}",
                    type_name="integer",
                    offset=layout.header_size + (len(layout.slots) * FRAME_WORD_SIZE),
                    size=FRAME_WORD_SIZE,
                )
        )

        with pytest.raises(CompileError, match="frame bytes plus 36 runtime bytes"):
            compile_algol(typed)

    def test_compiles_integer_array_descriptor_and_element_accesses(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; integer array a[1:3]; "
                "a[2] := 9; result := a[2] "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]
        data_labels = [decl.label for decl in result.program.data]

        assert result.heap_memory_label == "__algol_heap"
        assert data_labels == ["__algol_frames", "__algol_heap"]
        assert opcodes.count(IrOp.LOAD_WORD) >= 5
        assert opcodes.count(IrOp.STORE_WORD) >= 12
        assert IrOp.CMP_GT in opcodes

    def test_compiles_real_array_descriptor_and_element_accesses(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real array a[1:3]; "
                "a[2] := 1.5; "
                "if a[2] > 1.0 then result := 1 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert IrOp.STORE_F64 in opcodes
        assert IrOp.LOAD_F64 in opcodes
        assert IrOp.F64_CMP_GT in opcodes

    def test_compiles_dynamic_multidimensional_array_bounds(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result, lo, hi; "
                "lo := 2; hi := 4; "
                "begin integer array a[lo:hi, 1:2]; "
                "a[3, 2] := 11; result := a[3, 2] end "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert IrOp.DIV in opcodes
        assert opcodes.count(IrOp.MUL) >= 4
        assert opcodes.count(IrOp.CMP_GT) >= 6

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
        assert len(calls[0].operands) == 4
        assert any(label.startswith("_fn_algol_") for label in labels)
        assert result.procedure_signatures[calls[0].operands[0].name].param_count == 3

    def test_compiles_boolean_value_procedure_call(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "boolean procedure negate(x); value x; boolean x; "
                "begin negate := not x end; "
                "if negate(false) then result := 1 else result := 0 "
                "end"
            )
        )
        calls = [
            instruction
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert calls[0].operands[0].name.startswith("_fn_algol_")
        assert result.procedure_signatures[calls[0].operands[0].name].param_count == 3
        assert IrOp.AND_IMM in opcodes

    def test_compiles_real_value_procedure_call(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real y; "
                "real procedure half(x); value x; real x; "
                "begin half := x / 2 end; "
                "y := half(3); "
                "if y > 1.0 then result := 1 else result := 0 "
                "end"
            )
        )
        calls = [
            instruction
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        opcodes = [instr.opcode for instr in result.program.instructions]
        signature = result.procedure_signatures[calls[0].operands[0].name]

        assert signature.param_count == 3
        assert signature.param_types == ("integer", "integer", "real")
        assert signature.return_type == "real"
        assert IrOp.F64_DIV in opcodes
        assert IrOp.F64_CMP_GT in opcodes

    def test_compiles_integer_actual_promoted_for_real_value_parameter(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real y; "
                "real procedure id(x); value x; real x; "
                "begin id := x end; "
                "y := id(1); "
                "if y = 1.0 then result := 1 else result := 0 "
                "end"
            )
        )
        opcodes = [instr.opcode for instr in result.program.instructions]

        assert IrOp.F64_FROM_I32 in opcodes

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

    def test_compiles_scalar_by_name_parameter_as_storage_pointer(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "procedure bump(x); integer x; begin x := x + 1 end; "
                "result := 4; bump(result) "
                "end"
            )
        )
        calls = [
            instruction
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        opcodes = [instruction.opcode for instruction in result.program.instructions]

        assert len(calls[0].operands) == 4
        assert result.procedure_signatures[calls[0].operands[0].name].param_count == 3
        assert IrOp.ADD_IMM in opcodes
        assert opcodes.count(IrOp.LOAD_WORD) >= 4
        assert opcodes.count(IrOp.STORE_WORD) >= 8

    def test_compiles_boolean_by_name_parameter_as_storage_pointer(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "boolean flag; "
                "procedure settrue(x); boolean x; begin x := true end; "
                "flag := false; settrue(flag); "
                "if flag then result := 1 else result := 0 "
                "end"
            )
        )
        calls = [
            instruction
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        opcodes = [instruction.opcode for instruction in result.program.instructions]

        assert len(calls[0].operands) == 4
        assert result.procedure_signatures[calls[0].operands[0].name].param_count == 3
        assert opcodes.count(IrOp.LOAD_WORD) >= 4
        assert opcodes.count(IrOp.STORE_WORD) >= 8

    def test_compiles_read_only_by_name_expression_eval_thunk(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "integer procedure id(x); integer x; begin id := x end; "
                "result := id(1 + result) "
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

        assert "_fn_algol_eval_thunk" in labels
        assert result.procedure_signatures["_fn_algol_eval_thunk"].param_count == 2
        assert any(call.operands[0].name == "_fn_algol_eval_thunk" for call in calls)

    def test_compiles_read_only_real_by_name_expression_eval_thunk(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real y; "
                "real procedure id(x); real x; begin id := x end; "
                "y := id(1.5); "
                "if y > 1.0 then result := 1 else result := 0 "
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

        assert "_fn_algol_eval_real_thunk" in labels
        assert result.procedure_signatures["_fn_algol_eval_real_thunk"].param_types == (
            "integer",
            "integer",
        )
        assert (
            result.procedure_signatures["_fn_algol_eval_real_thunk"].return_type
            == "real"
        )
        assert any(call.operands[0].name == "_fn_algol_eval_real_thunk" for call in calls)

    def test_compiles_array_element_by_name_eval_and_store_thunks(
        self,
    ) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; integer array a[1:2]; "
                "procedure put(x); integer x; begin x := x + 1 end; "
                "a[1] := 6; "
                "put(a[1]); result := a[1] "
                "end"
            )
        )
        labels = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.LABEL
        ]
        calls = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]

        assert "_fn_algol_eval_thunk" in labels
        assert "_fn_algol_store_thunk" in labels
        assert "_fn_algol_eval_thunk" in calls
        assert "_fn_algol_store_thunk" in calls
        assert result.procedure_signatures["_fn_algol_eval_thunk"].param_count == 2
        assert result.procedure_signatures["_fn_algol_store_thunk"].param_count == 3

    def test_compiles_real_by_name_scalar_write_through_storage_pointer(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real y; "
                "procedure bump(x); real x; begin x := x + 1.5 end; "
                "y := 2.0; "
                "bump(y); "
                "if y > 3.0 then result := 1 else result := 0 "
                "end"
            )
        )
        opcodes = [instruction.opcode for instruction in result.program.instructions]

        assert IrOp.LOAD_F64 in opcodes
        assert IrOp.STORE_F64 in opcodes

    def test_compiles_real_array_element_by_name_eval_and_store_thunks(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; real array a[1:1]; "
                "procedure bump(x); real x; begin x := x + 1.5 end; "
                "a[1] := 2.0; "
                "bump(a[1]); "
                "if a[1] > 3.0 then result := 1 else result := 0 "
                "end"
            )
        )
        labels = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.LABEL
        ]
        calls = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]
        opcodes = [instruction.opcode for instruction in result.program.instructions]

        assert "_fn_algol_eval_real_thunk" in labels
        assert "_fn_algol_store_real_thunk" in labels
        assert "_fn_algol_eval_real_thunk" in calls
        assert "_fn_algol_store_real_thunk" in calls
        assert IrOp.LOAD_F64 in opcodes
        assert IrOp.STORE_F64 in opcodes

    def test_compiles_array_read_inside_expression_eval_thunk(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; integer array a[1:2]; "
                "integer procedure id(x); integer x; begin id := x end; "
                "result := id(a[1] + 1) "
                "end"
            )
        )
        calls = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]

        assert "_fn_algol_eval_thunk" in calls

    def test_compiles_procedure_call_inside_expression_eval_thunk(self) -> None:
        result = compile_algol(
            parse_algol(
                "begin integer result; "
                "integer procedure inc(n); value n; integer n; "
                "begin inc := n + 1 end; "
                "integer procedure id(x); integer x; begin id := x end; "
                "result := id(inc(4)) "
                "end"
            )
        )
        calls = [
            instruction.operands[0].name
            for instruction in result.program.instructions
            if instruction.opcode == IrOp.CALL
        ]

        assert "_fn_algol_eval_thunk" in calls
        assert any(label.startswith("_fn_algol_") for label in calls)
