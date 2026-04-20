"""Tests for the first ALGOL 60 type-checking subset."""

from algol_parser import parse_algol
from lang_parser import ASTNode

from algol_type_checker import (
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
