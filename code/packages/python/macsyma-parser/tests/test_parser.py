"""MACSYMA parser tests.

These verify that the grammar-driven parser produces the expected
``ASTNode`` shapes for representative MACSYMA programs. We don't pin
down every child position (that would make the tests brittle against
grammar tweaks); instead we check structural properties — root rule
name, number of statements, key nonterminal types appearing.
"""

from __future__ import annotations

from lang_parser import ASTNode, find_nodes
from macsyma_parser import parse_macsyma


def test_single_expression_statement() -> None:
    ast = parse_macsyma("x;")
    assert ast.rule_name == "program"
    statements = find_nodes(ast, "statement")
    assert len(statements) == 1


def test_arithmetic_expression() -> None:
    ast = parse_macsyma("1 + 2 * 3;")
    # Expect at least one `additive` and one `multiplicative` node,
    # since the grammar's precedence cascade uses both.
    assert len(find_nodes(ast, "additive")) >= 1
    assert len(find_nodes(ast, "multiplicative")) >= 1


def test_power_expression() -> None:
    ast = parse_macsyma("x^2;")
    powers = find_nodes(ast, "power")
    assert len(powers) >= 1


def test_parenthesized_expression() -> None:
    # `(1 + 2) * 3` should force the `additive` inside `multiplicative`
    # via the explicit `group` rule.
    ast = parse_macsyma("(1 + 2) * 3;")
    assert len(find_nodes(ast, "group")) >= 1


def test_function_call() -> None:
    ast = parse_macsyma("f(x, y);")
    postfix_nodes = find_nodes(ast, "postfix")
    assert len(postfix_nodes) >= 1


def test_assignment() -> None:
    ast = parse_macsyma("a : 5;")
    assign_nodes = find_nodes(ast, "assign")
    assert len(assign_nodes) >= 1


def test_function_definition() -> None:
    ast = parse_macsyma("f(x) := x^2;")
    assert ast.rule_name == "program"
    # The `assign` rule handles both `:` and `:=`, so we expect an
    # assign node in the tree.
    assert len(find_nodes(ast, "assign")) >= 1


def test_list_literal() -> None:
    ast = parse_macsyma("[1, 2, 3];")
    lists = find_nodes(ast, "list")
    assert len(lists) >= 1


def test_multiple_statements() -> None:
    ast = parse_macsyma("a : 5; b : 10; a + b;")
    statements = find_nodes(ast, "statement")
    assert len(statements) == 3


def test_suppress_output_dollar() -> None:
    # Both `;` and `$` are valid terminators.
    ast = parse_macsyma("x$")
    assert ast.rule_name == "program"
    assert len(find_nodes(ast, "statement")) == 1


def test_mixed_terminators() -> None:
    ast = parse_macsyma("a : 5$ b : 10; a + b;")
    assert len(find_nodes(ast, "statement")) == 3


def test_nested_function_calls() -> None:
    ast = parse_macsyma("diff(f(x), x);")
    # f(x) is one postfix with a call suffix; diff(...) is another.
    postfix_nodes = find_nodes(ast, "postfix")
    assert len(postfix_nodes) >= 2


def test_percent_constants() -> None:
    ast = parse_macsyma("%pi + %e;")
    # No crash; the atoms should parse as NAME tokens.
    assert ast.rule_name == "program"


def test_comparison() -> None:
    ast = parse_macsyma("x = 4;")
    assert len(find_nodes(ast, "comparison")) >= 1


def test_logical_operators() -> None:
    ast = parse_macsyma("a and b or not c;")
    assert len(find_nodes(ast, "logical_or")) >= 1
    assert len(find_nodes(ast, "logical_and")) >= 1


def test_unary_minus() -> None:
    ast = parse_macsyma("-x;")
    assert len(find_nodes(ast, "unary")) >= 1


def test_comment_ignored() -> None:
    ast = parse_macsyma("/* a note */ 42;")
    assert len(find_nodes(ast, "statement")) == 1


def test_empty_program() -> None:
    # Zero statements is valid — the grammar uses `{ statement }`.
    ast = parse_macsyma("")
    assert ast.rule_name == "program"
    assert len(find_nodes(ast, "statement")) == 0


def test_returns_ast_node() -> None:
    ast = parse_macsyma("1;")
    assert isinstance(ast, ASTNode)
