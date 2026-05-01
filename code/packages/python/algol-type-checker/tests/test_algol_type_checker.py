"""Tests for the first ALGOL 60 type-checking subset."""

from algol_parser import parse_algol
from lang_parser import ASTNode

from algol_type_checker import (
    FRAME_HEADER_SIZE,
    FRAME_REAL_SIZE,
    FRAME_WORD_SIZE,
    TypeCheckError,
    TypeCheckLimits,
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

    def test_rejects_ast_depth_past_configured_limit(self) -> None:
        ast = ASTNode("leaf", [])
        for _ in range(4):
            ast = ASTNode("wrapper", [ast], start_line=1, start_column=1)

        result = check_algol(ast, limits=TypeCheckLimits(max_ast_depth=3))

        assert not result.ok
        assert "AST depth 4 exceeds configured limit 3" in result.diagnostics[0].message
        assert result.semantic is not None
        assert result.semantic.root_block is None

    def test_rejects_block_nesting_past_configured_limit(self) -> None:
        ast = parse_algol(
            "begin begin integer a; begin integer x; x := 1 end end end"
        )

        result = check_algol(
            ast,
            limits=TypeCheckLimits(max_block_nesting_depth=1),
        )

        assert not result.ok
        assert "block nesting depth 2 exceeds configured limit 1" in (
            result.diagnostics[0].message
        )

    def test_rejects_procedure_nesting_past_configured_limit(self) -> None:
        ast = parse_algol(
            "begin "
            "procedure outer; "
            "begin procedure inner; begin integer x; x := 1 end; inner end; "
            "outer "
            "end"
        )

        result = check_algol(
            ast,
            limits=TypeCheckLimits(max_procedure_nesting_depth=1),
        )

        assert not result.ok
        assert "procedure nesting depth 2 exceeds configured limit 1" in (
            result.diagnostics[0].message
        )

    def test_rejects_procedure_body_block_past_configured_limit(self) -> None:
        ast = parse_algol(
            "begin procedure worker; begin integer x; x := 1 end; worker end"
        )

        result = check_algol(
            ast,
            limits=TypeCheckLimits(max_block_nesting_depth=0),
        )

        assert not result.ok
        assert "block nesting depth 1 exceeds configured limit 0" in (
            result.diagnostics[0].message
        )

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

    def test_accepts_go_to_spelling(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "go to done; "
            "result := 99; "
            "done: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.gotos[0].target_name == "done"

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

    def test_accepts_procedure_crossing_goto(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure escape; begin goto done end; "
            "escape; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.gotos[0].target_name == "done"

    def test_accepts_conditional_nonlocal_designational_goto(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "begin integer inner; goto if true then done else done end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None

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
        assert "switch index must be integer" in result.diagnostics[0].message

    def test_accepts_nonlocal_switch_selection(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "switch s := done; "
            "begin integer inner; goto s[1] end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert len(result.semantic.switch_selections) == 1
        selection = result.semantic.switch_selections[0]
        assert selection.use_block_id != selection.declaration_block_id
        assert selection.lexical_depth_delta == 1

    def test_accepts_switch_entry_targeting_nonlocal_label(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "begin switch s := done; goto s[1] end; "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert len(result.semantic.switches) == 1

    def test_accepts_nested_switch_selection_entries(self) -> None:
        ast = parse_algol(
            "begin integer result, i; "
            "switch inner := first, second; "
            "switch outer := inner[i]; "
            "i := 2; goto outer[1]; "
            "first: result := 1; goto done; "
            "second: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert len(result.semantic.switch_selections) == 2

    def test_accepts_self_recursive_switch_selection_entry(self) -> None:
        ast = parse_algol(
            "begin integer result, i; "
            "switch s := done, if i = 0 then done else s[i]; "
            "i := 1; goto s[2]; "
            "done: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert len(result.semantic.switch_selections) == 2

    def test_accepts_nonlocal_conditional_designational_goto(self) -> None:
        ast = parse_algol(
            "begin integer result, flag; "
            "begin goto if flag = 0 then left else right end; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok

    def test_accepts_procedure_crossing_conditional_designational_goto(self) -> None:
        ast = parse_algol(
            "begin integer result, flag; "
            "procedure escape; begin goto if flag = 0 then left else right end; "
            "flag := 1; escape; result := 0; "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
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

    def test_accepts_real_variable_declaration_and_assignment(self) -> None:
        ast = parse_algol(
            "begin integer result; real x; x := 1.5; "
            "if x > 1.0 then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)
        assert result.ok
        assert result.root_scope.children[0].symbols["x"].type_name == "real"

    def test_accepts_boolean_variable_declaration_and_assignment(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "boolean flag; "
            "flag := true; "
            "if flag then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.root_scope.children[0].symbols["flag"].type_name == "boolean"

    def test_reports_arithmetic_operand_that_is_not_integer(self) -> None:
        ast = parse_algol("begin integer result; result := true + 1 end")
        result = check_algol(ast)
        assert not result.ok
        assert "operator requires numeric operand" in result.diagnostics[0].message

    def test_accepts_boolean_operators_in_condition(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "if true and not false then result := 1 else result := 0 "
            "end"
        )
        assert check_algol(ast).ok

    def test_accepts_boolean_implication_and_equivalence(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "if (true impl false) eqv false then result := 1 else result := 0 "
            "end"
        )
        assert check_algol(ast).ok

    def test_accepts_chained_assignment(self) -> None:
        ast = parse_algol("begin integer result, other; result := other := 1 end")
        result = check_algol(ast)
        assert result.ok

    def test_accepts_real_division_and_integer_to_real_assignment(self) -> None:
        ast = parse_algol(
            "begin integer result; real x; x := 1 / 2; "
            "if x < 1.0 then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)
        assert result.ok

    def test_accepts_integer_exponentiation(self) -> None:
        ast = parse_algol("begin integer result; result := 2 ** 3 end")
        result = check_algol(ast)
        assert result.ok

    def test_accepts_real_exponentiation(self) -> None:
        ast = parse_algol("begin integer result; real x; x := 4.0 ** 0.5 end")
        result = check_algol(ast)
        assert result.ok

    def test_accepts_standard_numeric_builtin_functions(self) -> None:
        ast = parse_algol(
            "begin integer result; real x, root; "
            "root := sqrt(9); "
            "root := sin(root) + cos(0) + arctan(1) + ln(exp(1)); "
            "result := abs(0 - 3) + sign(x) + entier(root) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        calls = {
            call.name: call.return_type
            for call in result.semantic.procedure_calls
        }
        assert calls["abs"] == "integer"
        assert calls["sign"] == "integer"
        assert calls["entier"] == "integer"
        assert calls["sqrt"] == "real"
        assert calls["sin"] == "real"
        assert calls["cos"] == "real"
        assert calls["arctan"] == "real"
        assert calls["ln"] == "real"
        assert calls["exp"] == "real"

    def test_rejects_boolean_actual_for_numeric_builtin_function(self) -> None:
        ast = parse_algol("begin integer result; result := abs(false) end")
        result = check_algol(ast)

        assert not result.ok
        assert "builtin function 'abs' expects integer or real" in (
            result.diagnostics[0].message
        )

    def test_rejects_non_numeric_exponent_for_exponentiation(self) -> None:
        ast = parse_algol("begin real result; result := 2.0 ** false end")
        result = check_algol(ast)
        assert not result.ok
        assert (
            "operator requires numeric operand, got boolean"
            in result.diagnostics[0].message
        )

    def test_accepts_conditional_expression_assignment(self) -> None:
        ast = parse_algol("begin integer result; result := if true then 1 else 2 end")
        result = check_algol(ast)
        assert result.ok

    def test_accepts_mixed_numeric_conditional_expression_as_real(self) -> None:
        ast = parse_algol(
            "begin integer result; real x; "
            "x := if false then 1 else 2.5; "
            "if x > 2.0 then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)
        assert result.ok

    def test_rejects_incompatible_conditional_expression_branches(self) -> None:
        ast = parse_algol(
            "begin integer result; result := if true then 1 else false end"
        )
        result = check_algol(ast)
        assert not result.ok
        assert (
            "conditional expression branches must have compatible types"
            in result.diagnostics[0].message
        )

    def test_accepts_simple_for_element(self) -> None:
        ast = parse_algol("begin integer result, i; for i := 1 do result := i end")
        assert check_algol(ast).ok

    def test_accepts_while_for_element(self) -> None:
        ast = parse_algol(
            "begin integer result, i, x; x := 3; "
            "for i := x while x > 0 do begin result := result + i; x := x - 1 end "
            "end"
        )
        assert check_algol(ast).ok

    def test_accepts_multiple_for_elements_and_real_control(self) -> None:
        ast = parse_algol(
            "begin integer result; real x; "
            "for x := 1.5 step -0.5 until 0.5, 2.0 do result := result + 1 "
            "end"
        )
        assert check_algol(ast).ok

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

    def test_plans_real_scalar_frame_slot_with_eight_byte_size(self) -> None:
        ast = parse_algol("begin integer result; real x; result := 0 end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.root_block is not None
        layout = result.semantic.root_block.frame_layout
        expected_size = FRAME_HEADER_SIZE + FRAME_WORD_SIZE + FRAME_REAL_SIZE
        assert layout.frame_size == expected_size
        assert [(slot.name, slot.offset, slot.size) for slot in layout.slots] == [
            ("result", 20, FRAME_WORD_SIZE),
            ("x", 24, FRAME_REAL_SIZE),
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

    def test_own_scalar_uses_static_storage_outside_frame_layout(self) -> None:
        ast = parse_algol(
            "begin own integer counter; integer result; result := counter end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        counter = next(
            symbol for symbol in result.semantic.symbols if symbol.name == "counter"
        )
        result_symbol = next(
            symbol for symbol in result.semantic.symbols if symbol.name == "result"
        )
        assert counter.storage_class == "static"
        assert counter.slot_offset == 0
        assert result_symbol.storage_class == "frame"
        assert result.semantic.root_block is not None
        layout = result.semantic.root_block.frame_layout
        assert [slot.name for slot in layout.slots] == ["result"]

        counter_read = next(
            ref
            for ref in result.semantic.references
            if ref.name == "counter" and ref.role == "read"
        )
        assert counter_read.storage_class == "static"
        assert counter_read.slot_offset == 0

    def test_own_scalar_remains_lexically_visible_inside_nested_procedure(self) -> None:
        ast = parse_algol(
            "begin own integer counter; integer result; "
            "procedure bump; begin counter := counter + 1; result := counter end; "
            "bump "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        reads = [
            ref
            for ref in result.semantic.references
            if ref.name == "counter" and ref.role == "read"
        ]
        writes = [
            ref
            for ref in result.semantic.references
            if ref.name == "counter" and ref.role == "write"
        ]
        assert reads
        assert writes
        assert all(ref.storage_class == "static" for ref in [*reads, *writes])
        assert all(ref.lexical_depth_delta == 1 for ref in [*reads, *writes])

    def test_own_array_uses_static_descriptor_storage(self) -> None:
        ast = parse_algol(
            "begin own integer array counts[1:2]; integer result; "
            "counts[1] := 7; result := counts[1] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        counts = next(
            symbol for symbol in result.semantic.symbols if symbol.name == "counts"
        )
        descriptor = next(
            array for array in result.semantic.arrays if array.name == "counts"
        )
        assert counts.storage_class == "static"
        assert counts.slot_offset == 0
        assert counts.slot_size == 4
        assert descriptor.storage_class == "static"
        assert result.semantic.root_block is not None
        layout = result.semantic.root_block.frame_layout
        assert [slot.name for slot in layout.slots] == ["result"]

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

    def test_accepts_bare_no_argument_typed_procedure_expression(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure seven; begin seven := 7 end; "
            "result := seven "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        call = next(
            call
            for call in result.semantic.procedure_calls
            if call.name == "seven" and call.role == "expression"
        )
        assert call.argument_count == 0
        assert call.return_type == "integer"
        assert not any(
            reference.name == "seven" and reference.role == "read"
            for reference in result.semantic.references
        )

    def test_rejects_bare_typed_procedure_expression_with_required_argument(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure inc(x); value x; integer x; begin inc := x + 1 end; "
            "result := inc "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "procedure 'inc' expects 1 argument(s), got 0" in result.diagnostics[
            0
        ].message

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

    def test_accepts_boolean_value_procedure_signature_and_call(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "boolean procedure negate(x); value x; boolean x; "
            "begin negate := not x end; "
            "if negate(false) then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        descriptor = result.semantic.procedures[0]
        assert descriptor.return_type == "boolean"
        assert descriptor.parameters[0].type_name == "boolean"
        call = result.semantic.procedure_calls[0]
        assert call.return_type == "boolean"

    def test_accepts_real_value_procedure_signature_and_call(self) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "real procedure half(x); value x; real x; "
            "begin half := x / 2 end; "
            "y := half(3); "
            "if y > 1.0 then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        descriptor = result.semantic.procedures[0]
        assert descriptor.return_type == "real"
        assert descriptor.parameters[0].type_name == "real"
        call = result.semantic.procedure_calls[0]
        assert call.return_type == "real"

    def test_accepts_boolean_by_name_parameter_writeback(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "boolean flag; "
            "procedure settrue(x); boolean x; begin x := true end; "
            "flag := false; settrue(flag); "
            "if flag then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "boolean"
        assert parameter.mode == "name"
        assert parameter.may_write

    def test_accepts_integer_array_parameter_and_whole_array_actual(self) -> None:
        ast = parse_algol(
            "begin integer array xs[1:2]; integer result; "
            "procedure setfirst(a); integer a; array a; begin a[1] := 9 end; "
            "xs[1] := 4; setfirst(xs); result := xs[1] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "array"
        assert parameter.type_name == "integer"
        assert any(access.role == "actual" for access in result.semantic.array_accesses)

    def test_accepts_value_array_parameter_copy_mode(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer array xs[1:2]; "
            "procedure probe(a); value a; integer a; array a; begin a[1] := 9 end; "
            "probe(xs); result := xs[1] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "array"
        assert parameter.mode == "value"

    def test_accepts_label_parameter_and_direct_label_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure jump(target); label target; begin goto target end; "
            "jump(done); "
            "done: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "label"
        assert parameter.type_name == "label"

    def test_accepts_label_parameter_and_conditional_label_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; boolean flag; "
            "procedure jump(target); label target; begin goto target end; "
            "flag := false; jump(if flag then left else right); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "label"

    def test_accepts_label_parameter_and_numeric_label_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure jump(target); label target; begin goto target end; "
            "jump(10); "
            "10: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert any(label.name == "10" for label in result.semantic.labels)

    def test_accepts_label_parameter_and_switch_selection_actual(self) -> None:
        ast = parse_algol(
            "begin integer result, i; switch s := left, right; "
            "procedure jump(target); label target; begin goto target end; "
            "i := 2; jump(s[i]); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert any(
            selection.name == "s"
            for selection in result.semantic.switch_selections
        )

    def test_accepts_value_label_parameter_and_direct_label_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure jump(target); value target; label target; "
            "begin goto target end; "
            "jump(done); "
            "done: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "label"
        assert parameter.mode == "value"

    def test_accepts_switch_parameter_and_direct_switch_actual(self) -> None:
        ast = parse_algol(
            "begin integer result, flag; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "switch s := if flag = 0 then left else right; "
            "flag := 1; escape(s); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "switch"
        assert parameter.type_name == "switch"
        assert any(
            selection.name == "sw" and selection.switch_id == -1
            for selection in result.semantic.switch_selections
        )

    def test_accepts_switch_parameter_and_conditional_switch_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; boolean flag; "
            "switch a := left; switch b := right; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "flag := false; escape(if flag then a else b); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "switch"
        assert parameter.type_name == "switch"

    def test_rejects_switch_parameter_conditional_actual_with_non_boolean_condition(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result, flag; "
            "switch a := done; switch b := done; "
            "procedure escape(sw); switch sw; begin goto sw[1] end; "
            "escape(if flag then a else b); "
            "done: result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert (
            "switch designator actual condition must be boolean"
            in result.diagnostics[0].message
        )

    def test_accepts_value_switch_parameter_and_direct_switch_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure escape(sw); value sw; switch sw; begin goto sw[1] end; "
            "switch s := done; "
            "escape(s); result := 0; "
            "done: result := 7 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "switch"
        assert parameter.mode == "value"

    def test_accepts_procedure_parameter_and_direct_procedure_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure twice(p); procedure p; begin p; p end; "
            "procedure bump; begin result := result + 1 end; "
            "result := 0; twice(bump) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "procedure"
        assert parameter.type_name == "procedure"
        formal_call = next(
            call
            for call in result.semantic.procedure_calls
            if call.name == "p"
        )
        assert formal_call.parameter_symbol_id == parameter.symbol_id

    def test_accepts_procedure_parameter_with_value_argument_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(7) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.procedure_call_shapes[0].argument_types == ("integer",)

    def test_accepts_typed_procedure_parameter_expression_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "procedure invoke(f); real f; procedure f; "
            "begin y := f(2); if y = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "real"
        assert parameter.procedure_call_shapes[0].return_type == "real"

    def test_accepts_typed_procedure_parameter_with_array_argument_actual(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; integer array a[1:2]; "
            "procedure invoke(f); integer f; procedure f; "
            "begin result := f(a) end; "
            "integer procedure first(xs); integer xs; array xs; "
            "begin first := xs[1] end; "
            "invoke(first) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        shape = parameter.procedure_call_shapes[0]
        assert shape.return_type == "integer"
        assert shape.argument_kinds == ("array",)
        assert shape.argument_types == ("integer",)

    def test_accepts_integer_procedure_actual_for_real_procedure_parameter(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "procedure invoke(f); real f; procedure f; "
            "begin y := f(2); if y = 4.0 then result := 1 else result := 0 end; "
            "integer procedure twice(x); value x; integer x; "
            "begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "real"
        assert parameter.procedure_call_shapes[0].return_type == "real"

    def test_rejects_procedure_parameter_actual_with_mismatched_arity(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert (
            "expects a no-argument statement procedure actual"
            in result.diagnostics[0].message
        )

    def test_accepts_procedure_parameter_actual_with_read_only_by_name_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(7) end; "
            "procedure set(x); integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.procedure_call_shapes[0].argument_assignable == (False,)

    def test_accepts_procedure_parameter_actual_with_array_element_by_name_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; integer array a[1:1]; "
            "procedure invoke(p); procedure p; begin p(a[1]) end; "
            "procedure set(x); integer x; begin x := 7 end; "
            "a[1] := 0; invoke(set); result := a[1] "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.procedure_call_shapes[0].argument_assignable == (True,)
        assert any(
            access.name == "a" and access.role == "write"
            for access in result.semantic.array_accesses
        )

    def test_accepts_procedure_parameter_actual_with_array_formal(self) -> None:
        ast = parse_algol(
            "begin integer result; integer array a[1:2]; "
            "procedure invoke(p); procedure p; begin p(a) end; "
            "procedure first(xs); integer xs; array xs; "
            "begin result := xs[1] end; "
            "invoke(first) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        shape = parameter.procedure_call_shapes[0]
        assert shape.argument_kinds == ("array",)
        assert shape.argument_types == ("integer",)
        assert any(access.role == "actual" for access in result.semantic.array_accesses)

    def test_rejects_procedure_parameter_actual_with_wrong_array_type(self) -> None:
        ast = parse_algol(
            "begin integer result; real array a[1:2]; "
            "procedure invoke(p); procedure p; begin p(a) end; "
            "procedure first(xs); integer xs; array xs; "
            "begin result := xs[1] end; "
            "invoke(first) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "passes real array" in result.diagnostics[0].message

    def test_accepts_procedure_parameter_actual_with_label_formal(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(done) end; "
            "procedure jump(l); label l; begin result := 9; goto l end; "
            "invoke(jump); done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[0].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("label",)
        assert shape.argument_types == ("label",)

    def test_accepts_procedure_parameter_actual_with_conditional_label_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; boolean flag; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag then left else right) end; "
            "procedure jump(l); label l; begin goto l end; "
            "flag := false; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[0].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("label",)
        assert shape.argument_types == ("label",)

    def test_accepts_procedure_parameter_actual_with_switch_selection_label_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result, i; switch s := left, right; "
            "procedure invoke(p); procedure p; begin p(s[i]) end; "
            "procedure jump(l); label l; begin goto l end; "
            "i := 2; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[0].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("label",)
        assert shape.argument_types == ("label",)
        assert any(
            selection.name == "s"
            for selection in result.semantic.switch_selections
        )

    def test_accepts_procedure_parameter_actual_with_switch_formal(self) -> None:
        ast = parse_algol(
            "begin integer result; switch s := done; "
            "procedure invoke(p); procedure p; begin p(s) end; "
            "procedure jump(sw); switch sw; begin goto sw[1] end; "
            "invoke(jump); done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[0].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("switch",)
        assert shape.argument_types == ("switch",)

    def test_accepts_procedure_parameter_actual_with_conditional_switch_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; boolean flag; "
            "switch a := left; switch b := right; "
            "procedure invoke(p); procedure p; "
            "begin p(if flag then a else b) end; "
            "procedure jump(sw); switch sw; begin goto sw[1] end; "
            "flag := false; invoke(jump); "
            "left: result := 1; goto done; "
            "right: result := 2; "
            "done: "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[0].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("switch",)
        assert shape.argument_types == ("switch",)

    def test_accepts_procedure_parameter_actual_with_procedure_formal(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure bump; begin result := result + 1 end; "
            "procedure invoke(p); procedure p; begin p(bump) end; "
            "procedure use(q); procedure q; begin q end; "
            "invoke(use) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        shape = result.semantic.procedures[1].parameters[0].procedure_call_shapes[0]
        assert shape.argument_kinds == ("procedure",)
        assert shape.argument_types == ("procedure",)
        assert shape.procedure_argument_ids == (
            result.semantic.procedures[0].procedure_id,
        )

    def test_rejects_procedure_parameter_actual_with_nested_arity_mismatch(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure bump; begin result := result + 1 end; "
            "procedure invoke(p); procedure p; begin p(bump) end; "
            "procedure use(q); procedure q; begin q(1) end; "
            "invoke(use) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "accepting 1 argument(s), got 0" in result.diagnostics[0].message

    def test_rejects_procedure_parameter_actual_with_nested_result_mismatch(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure bump; begin result := result + 1 end; "
            "procedure invoke(p); procedure p; begin p(bump) end; "
            "procedure use(f); integer f; procedure f; begin result := f end; "
            "invoke(use) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "expects a integer procedure actual" in result.diagnostics[0].message

    def test_rejects_procedure_parameter_actual_with_wrong_label_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(done) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set); done: "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "passes a label" in result.diagnostics[0].message

    def test_rejects_procedure_parameter_actual_with_written_literal_by_name_formal(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); procedure p; begin p(7) end; "
            "procedure set(x); integer x; begin x := x + 1 end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "non-assignable actual" in result.diagnostics[0].message

    def test_rejects_typed_procedure_parameter_void_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "procedure invoke(f); real f; procedure f; begin y := f(2) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert "expects a real procedure actual" in result.diagnostics[0].message

    def test_accepts_value_procedure_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); value p; procedure p; begin p end; "
            "procedure bump; begin result := result + 1 end; "
            "result := 0; invoke(bump) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.kind == "procedure"
        assert parameter.mode == "value"

    def test_accepts_value_procedure_parameter_with_value_argument_actual(
        self,
    ) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure invoke(p); value p; procedure p; begin p(7) end; "
            "procedure set(x); value x; integer x; begin result := x end; "
            "invoke(set) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "value"
        assert parameter.procedure_call_shapes[0].argument_types == ("integer",)

    def test_accepts_value_typed_procedure_parameter_expression_actual(self) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "procedure invoke(f); value f; real f; procedure f; "
            "begin y := f(2); if y = 4 then result := 1 else result := 0 end; "
            "real procedure twice(x); value x; real x; begin twice := x * 2 end; "
            "invoke(twice) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "real"
        assert parameter.mode == "value"
        assert parameter.procedure_call_shapes[0].return_type == "real"

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

    def test_accepts_default_real_array_declaration(self) -> None:
        ast = parse_algol("begin array a[1:3]; a[1] := 7 end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.arrays[0].element_type == "real"

    def test_accepts_real_array_element_assignment(self) -> None:
        ast = parse_algol("begin real array a[1:3]; a[1] := 1.5 end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.arrays[0].element_type == "real"

    def test_accepts_boolean_array_element_assignment(self) -> None:
        ast = parse_algol(
            "begin integer result; boolean array flags[1:2]; "
            "flags[1] := true; "
            "if flags[1] then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.arrays[0].element_type == "boolean"

    def test_accepts_string_array_element_assignment(self) -> None:
        ast = parse_algol(
            "begin integer result; string array messages[1:2]; "
            "messages[1] := 'Hi'; print(messages[1]); result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.arrays[0].element_type == "string"

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

    def test_rejects_non_real_array_element_assignment(self) -> None:
        ast = parse_algol("begin real array a[1:3]; a[1] := false end")
        result = check_algol(ast)

        assert not result.ok
        assert "cannot assign boolean to real variable" in result.diagnostics[0].message

    def test_rejects_non_string_array_element_assignment(self) -> None:
        ast = parse_algol("begin string array messages[1:2]; messages[1] := 1 end")
        result = check_algol(ast)

        assert not result.ok
        assert (
            "cannot assign integer to string variable"
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

    def test_builtin_output_does_not_make_by_name_formal_writable(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "procedure emit(s); string s; begin print(s); result := 7 end; "
            "emit('Hi') "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "string"
        assert parameter.mode == "name"
        assert not parameter.may_write

    def test_accepts_boolean_expression_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; boolean flag; "
            "procedure test(b); boolean b; "
            "begin if b then result := 9 else result := 0 end; "
            "flag := false; test(not flag) "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.type_name == "boolean"
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

    def test_rejects_bare_procedure_actual_for_written_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "integer procedure seven; begin seven := 7 end; "
            "procedure put(x); integer x; begin x := 9 end; "
            "put(seven) "
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

    def test_accepts_real_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; real y; "
            "procedure bump(x); real x; begin x := x + 1.5 end; "
            "y := 1.5; bump(y); "
            "if y > 2.0 then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert parameter.type_name == "real"
        assert parameter.may_write

    def test_accepts_string_value_procedure_result(self) -> None:
        ast = parse_algol(
            "begin string msg; integer result; "
            "string procedure id(x); value x; string x; "
            "begin id := x end; "
            "msg := id('Hi'); print(msg); result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        procedure = result.semantic.procedures[0]
        assert procedure.return_type == "string"
        assert procedure.parameters[0].mode == "value"
        assert procedure.parameters[0].type_name == "string"

    def test_accepts_string_by_name_parameter(self) -> None:
        ast = parse_algol(
            "begin string msg; integer result; "
            "procedure setmsg(x); string x; begin x := 'OK' end; "
            "msg := 'Hi'; setmsg(msg); print(msg); result := 1 "
            "end"
        )
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        parameter = result.semantic.procedures[0].parameters[0]
        assert parameter.mode == "name"
        assert parameter.type_name == "string"
        assert parameter.may_write

    def test_rejects_wrong_type_for_boolean_value_parameter(self) -> None:
        ast = parse_algol(
            "begin integer result; "
            "boolean procedure negate(x); value x; boolean x; "
            "begin negate := not x end; "
            "if negate(1) then result := 1 else result := 0 "
            "end"
        )
        result = check_algol(ast)

        assert not result.ok
        assert (
            "parameter 'x' expects boolean, got integer"
            in result.diagnostics[0].message
        )

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

    def test_accepts_builtin_print_with_string_literal(self) -> None:
        ast = parse_algol("begin integer result; print('Hi'); result := 1 end")
        result = check_algol(ast)

        assert result.ok
        assert result.semantic is not None
        assert result.semantic.procedure_calls[-1].label == "__algol_builtin_print"

    def test_accepts_builtin_print_with_integer_argument(self) -> None:
        ast = parse_algol("begin integer result; print(1); result := 0 end")
        result = check_algol(ast)

        assert result.ok

    def test_rejects_builtin_print_in_expression(self) -> None:
        ast = parse_algol("begin integer result; result := print('Hi') end")
        result = check_algol(ast)

        assert not result.ok
        assert "does not return a value" in result.diagnostics[0].message

    def test_accepts_builtin_print_with_integer_expression_argument(self) -> None:
        ast = parse_algol("begin integer result; print(1 + 2); result := 1 end")
        result = check_algol(ast)

        assert result.ok

    def test_accepts_builtin_print_with_boolean_argument(self) -> None:
        ast = parse_algol(
            "begin integer result; print(1 < 2); result := 1 end"
        )
        result = check_algol(ast)

        assert result.ok

    def test_accepts_builtin_print_with_real_argument(self) -> None:
        ast = parse_algol("begin integer result; print(1.5); result := 0 end")
        result = check_algol(ast)

        assert result.ok

    def test_accepts_string_variable_declaration_and_assignment(self) -> None:
        ast = parse_algol("begin string msg; msg := 'Hi' end")
        result = check_algol(ast)

        assert result.ok
        assert result.root_scope.children[0].symbols["msg"].type_name == "string"

    def test_accepts_builtin_print_with_string_variable(self) -> None:
        ast = parse_algol(
            "begin string msg; integer result; msg := 'Hi'; "
            "print(msg); result := 1 end"
        )
        result = check_algol(ast)

        assert result.ok
