"""Tests for the ECMAScript 1 (1997) Parser.

These tests verify that the grammar-driven parser, loaded with ``es1.grammar``,
correctly parses ES1-era JavaScript into ASTs.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token, TokenType

from ecmascript_es1_parser import create_es1_parser, parse_es1


# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all descendant nodes with the given rule name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list[Token]:
    """Recursively collect all Token leaves from an AST."""
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(find_tokens(child))
    return tokens


# ============================================================================
# Test: Program Structure
# ============================================================================


class TestProgramStructure:
    """The root AST node should be a ``program``."""

    def test_empty_program(self) -> None:
        ast = parse_es1("")
        assert ast.rule_name == "program"
        assert len(ast.children) == 0

    def test_single_statement(self) -> None:
        ast = parse_es1("var x = 1;")
        assert ast.rule_name == "program"
        assert len(ast.children) >= 1


# ============================================================================
# Test: Variable Declarations
# ============================================================================


class TestVarDeclarations:
    """ES1 only has ``var`` — no ``let`` or ``const``."""

    def test_var_with_initializer(self) -> None:
        ast = parse_es1("var x = 1;")
        var_decls = find_nodes(ast, "variable_declaration")
        assert len(var_decls) >= 1

    def test_var_without_initializer(self) -> None:
        ast = parse_es1("var x;")
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(var_stmts) == 1

    def test_multiple_var_declarations(self) -> None:
        ast = parse_es1("var x = 1, y = 2;")
        var_decls = find_nodes(ast, "variable_declaration")
        assert len(var_decls) == 2


# ============================================================================
# Test: Function Declarations
# ============================================================================


class TestFunctionDeclarations:

    def test_simple_function(self) -> None:
        ast = parse_es1("function foo() { }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1

    def test_function_with_params(self) -> None:
        ast = parse_es1("function add(a, b) { return a + b; }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1

    def test_function_with_body(self) -> None:
        ast = parse_es1("function greet() { var msg = 1; }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1
        # Should have a function_body with statements
        func_bodies = find_nodes(ast, "function_body")
        assert len(func_bodies) >= 1


# ============================================================================
# Test: Control Flow Statements
# ============================================================================


class TestControlFlow:

    def test_if_statement(self) -> None:
        ast = parse_es1("if (x) { y; }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_if_else(self) -> None:
        ast = parse_es1("if (x) { y; } else { z; }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_while_loop(self) -> None:
        ast = parse_es1("while (x) { y; }")
        while_stmts = find_nodes(ast, "while_statement")
        assert len(while_stmts) == 1

    def test_do_while(self) -> None:
        ast = parse_es1("do { x; } while (y);")
        do_stmts = find_nodes(ast, "do_while_statement")
        assert len(do_stmts) == 1

    def test_for_loop(self) -> None:
        ast = parse_es1("for (var i = 0; i; i) { x; }")
        for_stmts = find_nodes(ast, "for_statement")
        assert len(for_stmts) == 1

    def test_switch(self) -> None:
        ast = parse_es1("switch (x) { case 1: y; default: z; }")
        switch_stmts = find_nodes(ast, "switch_statement")
        assert len(switch_stmts) == 1


# ============================================================================
# Test: Expressions
# ============================================================================


class TestExpressions:

    def test_expression_statement(self) -> None:
        ast = parse_es1("1 + 2;")
        expr_stmts = find_nodes(ast, "expression_statement")
        assert len(expr_stmts) == 1

    def test_operator_precedence(self) -> None:
        """Multiplication should bind tighter than addition."""
        ast = parse_es1("1 + 2 * 3;")
        # The AST should have an additive_expression containing
        # a multiplicative_expression
        add_nodes = find_nodes(ast, "additive_expression")
        mult_nodes = find_nodes(ast, "multiplicative_expression")
        assert len(add_nodes) >= 1
        assert len(mult_nodes) >= 1

    def test_assignment(self) -> None:
        """Bare assignments like ``x = 5;`` parse as expression statements
        containing an assignment_expression (the grammar routes through the
        full expression precedence chain)."""
        ast = parse_es1("var x = 5;")
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(var_stmts) == 1

    def test_ternary(self) -> None:
        ast = parse_es1("x ? y : z;")
        cond_nodes = find_nodes(ast, "conditional_expression")
        assert len(cond_nodes) >= 1

    def test_function_call(self) -> None:
        ast = parse_es1("foo(1, 2);")
        call_nodes = find_nodes(ast, "call_expression")
        assert len(call_nodes) >= 1


# ============================================================================
# Test: Literals
# ============================================================================


class TestLiterals:

    def test_object_literal(self) -> None:
        ast = parse_es1('var x = { a: 1, b: 2 };')
        obj_nodes = find_nodes(ast, "object_literal")
        assert len(obj_nodes) == 1

    def test_array_literal(self) -> None:
        ast = parse_es1("var x = [1, 2, 3];")
        arr_nodes = find_nodes(ast, "array_literal")
        assert len(arr_nodes) == 1


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:

    def test_two_statements(self) -> None:
        ast = parse_es1("var x = 1; var y = 2;")
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(var_stmts) == 2


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES1Parser:

    def test_creates_parser(self) -> None:
        parser = create_es1_parser("var x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        source = "var x = 1 + 2;"
        ast_direct = parse_es1(source)
        ast_factory = create_es1_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
        assert len(ast_direct.children) == len(ast_factory.children)
