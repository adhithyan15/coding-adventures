from __future__ import annotations

from excel_parser import create_excel_parser, parse_excel_formula


def test_parse_function_call_formula() -> None:
    ast = parse_excel_formula("=SUM(A1:B2)")
    assert ast.rule_name == "formula"


def test_parse_column_range_formula() -> None:
    ast = parse_excel_formula("A:C")
    assert ast.rule_name == "formula"


def test_parse_row_range_formula() -> None:
    ast = parse_excel_formula("1:3")
    assert ast.rule_name == "formula"


def test_factory_creates_parser() -> None:
    parser = create_excel_parser("A1")
    assert parser.parse().rule_name == "formula"
