from __future__ import annotations

from excel_lexer import create_excel_lexer, tokenize_excel_formula


def token_types(source: str) -> list[str]:
    return [token.type_name for token in tokenize_excel_formula(source)]


def test_function_names_are_reclassified() -> None:
    assert token_types("=SUM(A1)")[:3] == ["EQUALS", "FUNCTION_NAME", "LPAREN"]


def test_table_names_are_reclassified() -> None:
    assert token_types("DeptSales[Sales Amount]")[0] == "TABLE_NAME"


def test_factory_matches_direct_tokenize() -> None:
    source = "A1 + 1"
    assert create_excel_lexer(source).tokenize() == tokenize_excel_formula(source)
