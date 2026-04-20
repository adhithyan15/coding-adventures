"""Tests for the first ALGOL 60 type-checking subset."""

from algol_parser import parse_algol
from lang_parser import ASTNode

from algol_type_checker import (
    FRAME_HEADER_SIZE,
    FRAME_WORD_SIZE,
    TypeCheckError,
    __version__,
    assert_algol_typed,
    check_algol,
)


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestAlgolTypeChecker:
    """The checker accepts structured integer programs and rejects bad names."""

    def test_accepts_minimal_integer_result_program(self) -> None:
        ast = parse_algol("begin integer result; result := 7 end")
        result = check_algol(ast)
        assert result.ok
        assert result.root_scope.children[0].symbols["result"].type_name == "integer"

    def test_reports_undeclared_identifier(self) -> None:
        ast = parse_algol("begin integer result; result := missing end")
        result = check_algol(ast)
        assert not result.ok
        assert "not declared" in result.diagnostics[0].message

    def test_reports_assignment_type_mismatch(self) -> None:
        ast = parse_algol("begin integer result; result := true end")
        result = check_algol(ast)
        assert not result.ok
        assert "cannot assign boolean" in result.diagnostics[0].message

    def test_assert_helper_raises_with_locations(self) -> None:
        ast = parse_algol("begin integer result; result := false end")
        try:
            assert_algol_typed(ast)
        except TypeCheckError as exc:
            assert "Line 1, Col" in str(exc)
        else:  # pragma: no cover
            raise AssertionError("assert_algol_typed should reject false assignment")

    def test_accepts_if_else_and_for_loop(self) -> None:
        ast = parse_algol(
            "begin integer result, i; "
            "if 1 < 2 then result := 1 else result := 2; "
            "for i := 1 step 1 until 3 do result := result + i "
            "end"
        )
        result = check_algol(ast)
        assert result.ok

    def test_reports_missing_program_block(self) -> None:
        result = check_algol(ASTNode("program", []))
        assert not result.ok
        assert "must contain a block" in result.diagnostics[0].message

    def test_reports_redeclaration(self) -> None:
        ast = parse_algol("begin integer result; integer result; result := 0 end")
        result = check_algol(ast)
        assert not result.ok
        assert "already declared" in result.diagnostics[0].message

    def test_reports_unsupported_real_declaration(self) -> None:
        ast = parse_algol("begin real result; result := 1 end")
        result = check_algol(ast)
        assert not result.ok
        assert "real variables are not supported" in result.diagnostics[0].message

    def test_reports_arithmetic_operand_that_is_not_integer(self) -> None:
        ast = parse_algol("begin integer result; result := true + 1 end")
        result = check_algol(ast)
        assert not result.ok
        assert "operator requires integer" in result.diagnostics[0].message

    def test_accepts_boolean_operators_in_condition(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "if true and not false then result := 1 else result := 0 "
            "end"
        )
        assert check_algol(ast).ok

    def test_reports_chained_assignment(self) -> None:
        ast = parse_algol("begin integer result, other; result := other := 1 end")
        result = check_algol(ast)
        assert not result.ok
        assert "chained assignment" in result.diagnostics[0].message

    def test_reports_unsupported_real_division(self) -> None:
        ast = parse_algol("begin integer result; result := 1 / 2 end")
        result = check_algol(ast)
        assert not result.ok
        assert "real division is not supported" in result.diagnostics[0].message

    def test_reports_unsupported_exponentiation(self) -> None:
        ast = parse_algol("begin integer result; result := 2 ** 3 end")
        result = check_algol(ast)
        assert not result.ok
        assert "exponentiation is not supported" in result.diagnostics[0].message

    def test_reports_unsupported_simple_for_element(self) -> None:
        ast = parse_algol("begin integer result, i; for i := 1 do result := i end")
        result = check_algol(ast)
        assert not result.ok
        assert "only step/until" in result.diagnostics[0].message

    def test_accepts_nested_block_scope(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "begin integer inner; inner := 2; result := inner end "
            "end"
        )
        assert check_algol(ast).ok

    def test_plans_root_block_scalar_frame_slots(self) -> None:
        ast = parse_algol("begin integer result, other; result := other end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.root_block is not None
        layout = result.semantic.root_block.frame_layout
        assert layout.block_id == 0
        assert layout.depth == 0
        assert layout.static_parent_id is None
        assert layout.frame_size == FRAME_HEADER_SIZE + (2 * FRAME_WORD_SIZE)
        assert [(slot.name, slot.offset) for slot in layout.slots] == [
            ("result", 20),
            ("other", 24),
        ]

    def test_records_outer_scope_reference_static_link_delta(self) -> None:
        ast = parse_algol(
            "begin integer outer; "
            "begin integer inner; inner := outer end "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        outer_read = next(
            ref
            for ref in result.semantic.references
            if ref.name == "outer" and ref.role == "read"
        )
        assert outer_read.use_block_id == 1
        assert outer_read.declaration_block_id == 0
        assert outer_read.lexical_depth_delta == 1
        assert outer_read.slot_offset == FRAME_HEADER_SIZE

    def test_shadowing_resolves_to_nearest_frame_slot(self) -> None:
        ast = parse_algol(
            "begin integer x, sink; "
            "begin integer x; x := 2; sink := x end "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        outer_x = next(
            symbol
            for symbol in result.semantic.symbols
            if symbol.name == "x" and symbol.declaring_block_id == 0
        )
        inner_x = next(
            symbol
            for symbol in result.semantic.symbols
            if symbol.name == "x" and symbol.declaring_block_id == 1
        )
        inner_write = next(
            ref
            for ref in result.semantic.references
            if ref.name == "x" and ref.role == "write" and ref.use_block_id == 1
        )
        inner_read = next(
            ref
            for ref in result.semantic.references
            if ref.name == "x" and ref.role == "read" and ref.use_block_id == 1
        )
        assert inner_write.symbol_id == inner_x.symbol_id
        assert inner_read.symbol_id == inner_x.symbol_id
        assert inner_write.symbol_id != outer_x.symbol_id
        assert inner_write.lexical_depth_delta == 0

    def test_accepts_integer_value_procedure_descriptor(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure inc(x); value x; integer x; "
            "begin inc := x + 1 end; "
            "result := inc(4) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        descriptor = result.semantic.procedures[0]
        assert descriptor.name == "inc"
        assert descriptor.return_type == "integer"
        assert descriptor.parameters[0].name == "x"
        assert descriptor.parameters[0].mode == "value"
        body_block = result.semantic.blocks[descriptor.body_block_id]
        assert body_block.scope.symbols["inc"].kind == "procedure_result"
        assert body_block.scope.symbols["x"].kind == "parameter"

    def test_records_procedure_call_static_link_delta(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure outer(x); value x; integer x; "
            "begin integer procedure inner(y); value y; integer y; "
            "begin inner := x + y end; "
            "outer := inner(3) end; "
            "result := outer(4) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        inner_call = next(
            call for call in result.semantic.procedure_calls if call.name == "inner"
        )
        assert inner_call.lexical_depth_delta == 0
        outer_param_read = next(
            ref for ref in result.semantic.references if ref.name == "x"
        )
        assert outer_param_read.lexical_depth_delta == 1

    def test_accepts_void_procedure_statement_call(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure setresult(x); value x; integer x; "
            "begin result := x end; "
            "setresult(6) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        call = result.semantic.procedure_calls[0]
        assert call.role == "statement"
        assert call.return_type is None

    def test_accepts_integer_array_descriptor_and_accesses(self) -> None:
        ast = parse_algol(
            "begin integer result, lo, hi; "
            "integer array a[lo:hi, 1:3]; "
            "a[lo, 2] := 7; "
            "result := a[hi, 3] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        descriptor = result.semantic.arrays[0]
        assert descriptor.name == "a"
        assert descriptor.element_type == "integer"
        assert len(descriptor.dimensions) == 2
        assert descriptor.slot_offset == FRAME_HEADER_SIZE + (3 * FRAME_WORD_SIZE)
        assert [access.role for access in result.semantic.array_accesses] == [
            "write",
            "read",
        ]

    def test_rejects_array_without_integer_type_for_now(self) -> None:
        ast = parse_algol("begin array a[1:3]; a[1] := 7 end")
        result = check_algol(ast)

        assert not result.ok
        assert "real arrays are not supported" in result.diagnostics[0].message

    def test_rejects_wrong_array_subscript_count(self) -> None:
        ast = parse_algol(
            "begin integer result; integer array a[1:3, 1:3]; result := a[1] end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "expects 2 subscript" in result.diagnostics[0].message

    def test_rejects_non_integer_array_element_assignment(self) -> None:
        ast = parse_algol("begin integer array a[1:3]; a[1] := false end")
        result = check_algol(ast)

        assert not result.ok
        assert (
            "cannot assign boolean to integer variable"
            in result.diagnostics[0].message
        )

    def test_rejects_array_used_without_subscripts(self) -> None:
        ast = parse_algol("begin integer result; integer array a[1:3]; result := a end")
        result = check_algol(ast)

        assert not result.ok
        assert "requires subscripts" in result.diagnostics[0].message

    def test_rejects_call_by_name_parameter_for_now(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "call-by-name parameter" in result.diagnostics[0].message

    def test_rejects_procedure_argument_count_mismatch(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure id(x); value x; integer x; begin id := x end; "
            "result := id(1, 2) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "expects 1 argument" in result.diagnostics[0].message

    def test_rejects_void_procedure_in_expression(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure setresult(x); value x; integer x; begin result := x end; "
            "result := setresult(6) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "does not return a value" in result.diagnostics[0].message
