"""Tests for the ECMAScript 3 (1999) Parser.

ES3 adds try/catch/finally/throw, strict equality, instanceof, and regex
to the ES1 grammar.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token

from ecmascript_es3_parser import create_es3_parser, parse_es3


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode) -> list[Token]:
    tokens: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            tokens.append(child)
        elif isinstance(child, ASTNode):
            tokens.extend(find_tokens(child))
    return tokens


# ============================================================================
# Test: ES3 try/catch/finally (NEW)
# ============================================================================


class TestTryCatchFinally:

    def test_try_catch(self) -> None:
        ast = parse_es3("try { var x = 1; } catch (e) { var y = 2; }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) == 1

    def test_try_finally(self) -> None:
        ast = parse_es3("try { var x = 1; } finally { var y = 2; }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) == 1

    def test_try_catch_finally(self) -> None:
        ast = parse_es3("try { } catch (e) { } finally { }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) == 1

    def test_throw(self) -> None:
        ast = parse_es3('throw 1;')
        throw_stmts = find_nodes(ast, "throw_statement")
        assert len(throw_stmts) == 1


# ============================================================================
# Test: ES3 Strict Equality in Expressions
# ============================================================================


class TestStrictEqualityParsing:

    def test_strict_equals_expression(self) -> None:
        ast = parse_es3("var x = 1 === 2;")
        eq_nodes = find_nodes(ast, "equality_expression")
        assert len(eq_nodes) >= 1


# ============================================================================
# Test: ES1 Features Still Parse in ES3
# ============================================================================


class TestES1Compatibility:

    def test_var_declaration(self) -> None:
        ast = parse_es3("var x = 1;")
        assert ast.rule_name == "program"

    def test_function_declaration(self) -> None:
        ast = parse_es3("function foo() { }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1

    def test_if_else(self) -> None:
        ast = parse_es3("if (x) { } else { }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_while_loop(self) -> None:
        ast = parse_es3("while (x) { }")
        while_stmts = find_nodes(ast, "while_statement")
        assert len(while_stmts) == 1

    def test_for_loop(self) -> None:
        ast = parse_es3("for (var i = 0; i; i) { }")
        for_stmts = find_nodes(ast, "for_statement")
        assert len(for_stmts) == 1

    def test_switch(self) -> None:
        ast = parse_es3("switch (x) { case 1: var y = 2; }")
        switch_stmts = find_nodes(ast, "switch_statement")
        assert len(switch_stmts) == 1

    def test_object_literal(self) -> None:
        ast = parse_es3("var x = { a: 1 };")
        obj_nodes = find_nodes(ast, "object_literal")
        assert len(obj_nodes) == 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES3Parser:

    def test_creates_parser(self) -> None:
        parser = create_es3_parser("var x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        source = "try { } catch (e) { }"
        ast_direct = parse_es3(source)
        ast_factory = create_es3_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
