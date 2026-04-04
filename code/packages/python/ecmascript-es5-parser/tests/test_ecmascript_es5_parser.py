"""Tests for the ECMAScript 5 (2009) Parser.

ES5 adds the debugger statement and getter/setter properties on top of ES3.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token

from ecmascript_es5_parser import create_es5_parser, parse_es5


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


# ============================================================================
# Test: ES5 Debugger Statement (NEW)
# ============================================================================


class TestDebuggerStatement:

    def test_debugger(self) -> None:
        ast = parse_es5("debugger;")
        dbg_stmts = find_nodes(ast, "debugger_statement")
        assert len(dbg_stmts) == 1

    def test_debugger_in_function(self) -> None:
        ast = parse_es5("function foo() { debugger; }")
        dbg_stmts = find_nodes(ast, "debugger_statement")
        assert len(dbg_stmts) == 1


# ============================================================================
# Test: ES3 Features Still Parse in ES5
# ============================================================================


class TestES3Compatibility:

    def test_try_catch(self) -> None:
        ast = parse_es5("try { } catch (e) { }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) == 1

    def test_throw(self) -> None:
        ast = parse_es5("throw 1;")
        throw_stmts = find_nodes(ast, "throw_statement")
        assert len(throw_stmts) == 1


# ============================================================================
# Test: ES1 Features Still Parse in ES5
# ============================================================================


class TestES1Compatibility:

    def test_var_declaration(self) -> None:
        ast = parse_es5("var x = 1;")
        assert ast.rule_name == "program"

    def test_function_declaration(self) -> None:
        ast = parse_es5("function foo() { }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1

    def test_if_else(self) -> None:
        ast = parse_es5("if (x) { } else { }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_object_literal(self) -> None:
        ast = parse_es5("var x = { a: 1 };")
        obj_nodes = find_nodes(ast, "object_literal")
        assert len(obj_nodes) == 1


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:

    def test_debugger_and_var(self) -> None:
        ast = parse_es5("debugger; var x = 1;")
        dbg_stmts = find_nodes(ast, "debugger_statement")
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(dbg_stmts) == 1
        assert len(var_stmts) == 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateES5Parser:

    def test_creates_parser(self) -> None:
        parser = create_es5_parser("debugger;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        source = "debugger; var x = 1;"
        ast_direct = parse_es5(source)
        ast_factory = create_es5_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
