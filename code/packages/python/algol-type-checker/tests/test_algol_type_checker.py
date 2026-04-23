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

    def test_accepts_local_labels_and_direct_gotos(self) -> None:
        ast = parse_algol(
            "begin integer result, i; "
            "goto done; "
            "loop: i := i + 1; "
            "if i < 3 then goto loop; "
            "done: result := i "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert [label.name for label in result.semantic.labels] == ["loop", "done"]
        assert [goto.target_name for goto in result.semantic.gotos] == [
            "done",
            "loop",
        ]
        assert all(goto.lexical_depth_delta == 0 for goto in result.semantic.gotos)

    def test_accepts_label_on_empty_statement(self) -> None:
        ast = parse_algol("begin integer result; goto 10; 10: end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.labels[0].name == "10"
        assert result.semantic.gotos[0].target_name == "10"

    def test_rejects_missing_goto_label(self) -> None:
        ast = parse_algol("begin integer result; goto missing end")
        result = check_algol(ast)

        assert not result.ok
        assert "label 'missing' is not declared" in result.diagnostics[0].message

    def test_accepts_direct_nonlocal_block_goto(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "begin integer inner; goto done end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.gotos[0].target_name == "done"
        assert result.semantic.gotos[0].lexical_depth_delta == 1

    def test_rejects_procedure_crossing_goto_for_now(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure escape; begin goto done end; "
            "escape; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "crosses a procedure boundary" in result.diagnostics[0].message

    def test_rejects_conditional_nonlocal_designational_goto_for_now(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "begin integer inner; goto if true then done else done end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "nonlocal designational label 'done'" in result.diagnostics[0].message

    def test_accepts_conditional_designational_goto(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "goto if true then done else done; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.gotos[0].target_name == (
            "conditional designational expression"
        )

    def test_accepts_switch_declaration_and_designational_goto(self) -> None:
        ast = parse_algol(
            "begin integer result, i; "
            "switch s := first, second; "
            "i := 2; goto s[i]; "
            "first: result := 1; second: result := 2 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.switches[0].name == "s"
        assert len(result.semantic.switches[0].entry_node_ids) == 2
        assert result.semantic.switch_selections[0].name == "s"
        assert result.semantic.gotos[0].target_name == "switch designational expression"

    def test_rejects_missing_switch_designational_goto(self) -> None:
        ast = parse_algol("begin integer result; goto s[1] end")
        result = check_algol(ast)

        assert not result.ok
        assert "switch 's' is not declared" in result.diagnostics[0].message

    def test_rejects_unsupported_switch_index_expression(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "switch s := done; "
            "goto s[1 / 2]; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "real division is not supported" in result.diagnostics[0].message

    def test_rejects_nonlocal_switch_selection_for_now(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "switch s := done; "
            "begin integer inner; goto s[1] end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "nonlocal switch 's'" in result.diagnostics[0].message

    def test_rejects_nested_switch_selection_entries_for_now(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "switch s := s[1]; "
            "goto s[1] "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "select another switch require Phase 7b" in result.diagnostics[0].message

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

    def test_accepts_read_only_integer_call_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(1) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert not parameter.may_write

    def test_accepts_assignable_scalar_actual_for_written_by_name_parameter(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure inc(x); integer x; begin x := x + 1 end; "
            "inc(result) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert parameter.may_write
        assert parameter.write_reason == "local assignment"
        assert any(
            ref.name == "result" and ref.role == "write"
            for ref in result.semantic.references
        )

    def test_rejects_non_assignable_actual_for_written_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure inc(x); integer x; begin x := x + 1 end; "
            "inc(result + 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message

    def test_rejects_literal_actual_for_written_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure put(x); integer x; begin x := 7 end; "
            "put(1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message

    def test_rejects_procedure_call_actual_for_written_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure inc(n); value n; integer n; begin inc := n + 1 end; "
            "procedure put(x); integer x; begin x := 7 end; "
            "put(inc(1)) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message

    def test_accepts_array_element_actual_for_written_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; integer array a[1:2]; "
            "procedure put(x); integer x; begin x := 7 end; "
            "put(a[1]); result := a[1] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert any(access.role == "write" for access in result.semantic.array_accesses)

    def test_rejects_transitively_written_by_name_expression_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure inner(z); integer z; begin z := z + 1 end; "
            "procedure outer(x); integer x; begin inner(x) end; "
            "outer(result + 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message
        outer = next(
            procedure
            for procedure in result.semantic.procedures
            if procedure.name == "outer"
        )
        assert outer.parameters[0].write_reason == "transitive call"

    def test_read_only_transitive_call_keeps_by_name_formal_read_only(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure read(z); integer z; begin read := z end; "
            "integer procedure outer(x); integer x; begin outer := read(x) end; "
            "result := outer(1 + result) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        outer = next(
            procedure
            for procedure in result.semantic.procedures
            if procedure.name == "outer"
        )
        assert not outer.parameters[0].may_write

    def test_shadowed_read_only_procedure_call_is_conservative_write(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure read(z); integer z; begin read := z end; "
            "procedure outer(x); integer x; "
            "begin procedure read(y); integer y; begin y := y + 1 end; read(x) end; "
            "outer(result + 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message
        outer = next(
            procedure
            for procedure in result.semantic.procedures
            if procedure.name == "outer"
        )
        assert outer.parameters[0].write_reason == "transitive call"

    def test_shadowed_current_procedure_call_is_conservative_write(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure outer(x); integer x; "
            "begin procedure outer(y); integer y; begin y := y + 1 end; outer(x) end; "
            "outer(result + 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message
        outer = next(
            procedure
            for procedure in result.semantic.procedures
            if procedure.name == "outer"
        )
        assert outer.parameters[0].write_reason == "transitive call"

    def test_duplicate_procedure_names_do_not_hide_visible_writer(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure p(z); integer z; begin z := z + 1 end; "
            "procedure box(dummy); value dummy; integer dummy; "
            "begin integer procedure p(y); integer y; begin p := y end; "
            "result := p(dummy) end; "
            "procedure outer(x); integer x; begin p(x) end; "
            "outer(result + 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message
        outer = next(
            procedure
            for procedure in result.semantic.procedures
            if procedure.name == "outer"
        )
        assert outer.parameters[0].write_reason == "transitive call"

    def test_read_only_recursive_by_name_formal_stays_read_only(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure again(x, n); value n; integer x, n; "
            "begin if n = 0 then again := x else again := again(x, n - 1) end; "
            "result := again(1 + result, 1) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert not parameter.may_write

    def test_recursive_by_name_write_propagates_through_self_call(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure move(x, y, n); value n; integer x, y, n; "
            "begin if n = 0 then x := 1 else move(y, x, n - 1) end; "
            "move(result, result + 1, 1) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "actual expression is not assignable" in result.diagnostics[0].message

    def test_recursive_self_call_does_not_write_through_value_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure read(x, y, n); value y, n; integer x, y, n; "
            "begin if n = 0 then read := x "
            "else begin y := y + 1; read := read(x, x, n - 1) end end; "
            "result := read(1 + result, 0, 1) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        procedure = result.semantic.procedures[0]
        assert procedure.parameters[0].mode == "name"
        assert not procedure.parameters[0].may_write
        assert procedure.parameters[1].mode == "value"
        assert not procedure.parameters[1].may_write

    def test_shadowed_local_does_not_make_by_name_formal_writable(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure read(x); integer x; "
            "begin begin integer x; x := 4 end; read := x end; "
            "result := read(1 + result) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert not parameter.may_write

    def test_rejects_wrong_type_for_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure id(x); integer x; begin id := x end; "
            "result := id(false) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "by-name parameter 'x' expects integer" in result.diagnostics[0].message

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
