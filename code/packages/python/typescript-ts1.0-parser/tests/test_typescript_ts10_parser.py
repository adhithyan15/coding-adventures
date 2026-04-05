"""Tests for the TypeScript 1.0 (April 2014) Parser.

TypeScript 1.0 added interfaces, classes, enums, namespaces, type aliases,
and ambient declarations on top of ES5. The parser produces ASTs where each
node has a ``rule_name`` attribute identifying its grammar rule.

The root of every AST is a ``program`` node.
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts10_parser import create_ts10_parser, parse_ts10

# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with the given rule name.

    This helper traverses the entire AST tree and collects every node
    whose ``rule_name`` matches the given string. It is used in tests
    to verify that specific grammar constructs appear in the parse tree.

    Args:
        node: The root AST node to search from.
        rule_name: The grammar rule name to search for.

    Returns:
        A list of all matching ``ASTNode`` instances.
    """
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


# ============================================================================
# Test: Root Program Node
# ============================================================================


class TestProgramNode:
    """Every parse result must have 'program' as the root rule name."""

    def test_var_declaration_program_root(self) -> None:
        """``var x: number = 1;`` → root is program."""
        ast = parse_ts10("var x: number = 1;")
        assert ast.rule_name == "program"

    def test_empty_program(self) -> None:
        """An empty source produces a program node."""
        ast = parse_ts10("")
        assert ast.rule_name == "program"

    def test_simple_expression_program_root(self) -> None:
        ast = parse_ts10("1 + 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Typed Function Declaration
# ============================================================================


class TestTypedFunctionDeclaration:
    """Function declarations with type annotations.

    Example: ``function foo(x: string): string { return x; }``

    The parser should recognize this as a ``function_declaration``, with
    type annotations on the parameter and return type.
    """

    def test_typed_function_declaration(self) -> None:
        ast = parse_ts10("function foo(x: string): string { return x; }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1

    def test_untyped_function_still_works(self) -> None:
        ast = parse_ts10("function bar() { return 1; }")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) == 1


# ============================================================================
# Test: Interface Declaration
# ============================================================================


class TestInterfaceDeclaration:
    """Interface declarations define object shape contracts.

    Example: ``interface Foo { name: string; }``

    The grammar rule is ``interface_declaration``.
    """

    def test_basic_interface(self) -> None:
        ast = parse_ts10("interface Foo { name: string; }")
        iface_decls = find_nodes(ast, "interface_declaration")
        assert len(iface_decls) == 1

    def test_empty_interface(self) -> None:
        """An interface with no members is valid."""
        ast = parse_ts10("interface Empty {}")
        iface_decls = find_nodes(ast, "interface_declaration")
        assert len(iface_decls) == 1

    def test_interface_with_multiple_members(self) -> None:
        ast = parse_ts10("interface Point { x: number; y: number; }")
        iface_decls = find_nodes(ast, "interface_declaration")
        assert len(iface_decls) == 1

    def test_interface_with_method(self) -> None:
        ast = parse_ts10("interface Runnable { run(): void; }")
        iface_decls = find_nodes(ast, "interface_declaration")
        assert len(iface_decls) == 1


# ============================================================================
# Test: Type Alias Declaration
# ============================================================================


class TestTypeAliasDeclaration:
    """Type aliases give names to existing types.

    Example: ``type Alias = string;``

    The grammar rule is ``type_alias_declaration``.
    """

    def test_basic_type_alias(self) -> None:
        ast = parse_ts10("type Alias = string;")
        alias_decls = find_nodes(ast, "type_alias_declaration")
        assert len(alias_decls) == 1

    def test_union_type_alias(self) -> None:
        ast = parse_ts10("type StringOrNumber = string | number;")
        alias_decls = find_nodes(ast, "type_alias_declaration")
        assert len(alias_decls) == 1


# ============================================================================
# Test: Enum Declaration
# ============================================================================


class TestEnumDeclaration:
    """Enum declarations define named constant sets.

    Example: ``enum Color { Red, Green, Blue }``

    The grammar rule is ``enum_declaration``.
    """

    def test_basic_enum(self) -> None:
        ast = parse_ts10("enum Color { Red, Green, Blue }")
        enum_decls = find_nodes(ast, "enum_declaration")
        assert len(enum_decls) == 1

    def test_enum_with_values(self) -> None:
        ast = parse_ts10("enum Direction { Up = 1, Down = 2 }")
        enum_decls = find_nodes(ast, "enum_declaration")
        assert len(enum_decls) == 1

    def test_const_enum(self) -> None:
        ast = parse_ts10("const enum Status { Active, Inactive }")
        enum_decls = find_nodes(ast, "enum_declaration")
        assert len(enum_decls) == 1


# ============================================================================
# Test: Ambient Declaration
# ============================================================================


class TestAmbientDeclaration:
    """Ambient declarations tell TypeScript about external code.

    Example: ``declare var x: number;``

    The grammar rule is ``ambient_declaration``.
    """

    def test_declare_var(self) -> None:
        ast = parse_ts10("declare var x: number;")
        ambient_decls = find_nodes(ast, "ambient_declaration")
        assert len(ambient_decls) == 1

    def test_declare_function(self) -> None:
        ast = parse_ts10("declare function foo(x: string): void;")
        ambient_decls = find_nodes(ast, "ambient_declaration")
        assert len(ambient_decls) == 1


# ============================================================================
# Test: Class Declaration
# ============================================================================


class TestClassDeclaration:
    """Class declarations define reference types with typed members.

    Example: ``class Animal { name: string; }``

    Note: The TypeScript grammar uses ``ts_class_declaration`` to distinguish
    from a future ES2015 ``class_declaration`` rule.
    """

    def test_basic_class(self) -> None:
        ast = parse_ts10("class Animal { name: string; }")
        class_decls = find_nodes(ast, "ts_class_declaration")
        assert len(class_decls) == 1

    def test_class_with_constructor(self) -> None:
        ast = parse_ts10("class Point { constructor(x: number, y: number) {} }")
        class_decls = find_nodes(ast, "ts_class_declaration")
        assert len(class_decls) == 1

    def test_class_extends(self) -> None:
        ast = parse_ts10("class Dog extends Animal { bark(): void {} }")
        class_decls = find_nodes(ast, "ts_class_declaration")
        assert len(class_decls) == 1


# ============================================================================
# Test: Namespace Declaration
# ============================================================================


class TestNamespaceDeclaration:
    """Namespace declarations group related declarations.

    Example: ``namespace MyNS { }``

    The grammar rule is ``namespace_declaration``.
    """

    def test_empty_namespace(self) -> None:
        ast = parse_ts10("namespace MyNS { }")
        ns_decls = find_nodes(ast, "namespace_declaration")
        assert len(ns_decls) == 1

    def test_namespace_with_interface(self) -> None:
        ast = parse_ts10("namespace Shapes { interface Circle { radius: number; } }")
        ns_decls = find_nodes(ast, "namespace_declaration")
        assert len(ns_decls) == 1


# ============================================================================
# Test: ES5 Compatibility
# ============================================================================


class TestES5Compatibility:
    """TS 1.0 is a superset of ES5 — all ES5 programs must parse correctly."""

    def test_var_declaration(self) -> None:
        ast = parse_ts10("var x = 1;")
        assert ast.rule_name == "program"

    def test_if_else(self) -> None:
        ast = parse_ts10("if (x) { } else { }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) == 1

    def test_try_catch(self) -> None:
        ast = parse_ts10("try { } catch (e) { }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) == 1

    def test_debugger_statement(self) -> None:
        ast = parse_ts10("debugger;")
        dbg_stmts = find_nodes(ast, "debugger_statement")
        assert len(dbg_stmts) == 1


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:
    """Multiple TypeScript declarations in one source file."""

    def test_interface_and_class(self) -> None:
        ast = parse_ts10(
            "interface IFoo { x: number; }\n"
            "class Foo implements IFoo { x: number = 1; }"
        )
        iface_decls = find_nodes(ast, "interface_declaration")
        class_decls = find_nodes(ast, "ts_class_declaration")
        assert len(iface_decls) == 1
        assert len(class_decls) == 1

    def test_var_and_interface(self) -> None:
        ast = parse_ts10("var x: number = 1;\ninterface Foo { }")
        var_stmts = find_nodes(ast, "variable_statement")
        iface_decls = find_nodes(ast, "interface_declaration")
        assert len(var_stmts) == 1
        assert len(iface_decls) == 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS10Parser:
    """Tests for the ``create_ts10_parser`` factory function."""

    def test_creates_parser(self) -> None:
        """Factory returns an object with a parse method."""
        parser = create_ts10_parser("var x: number = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """Factory result matches direct parse_ts10 call."""
        source = "var x: number = 1;"
        ast_direct = parse_ts10(source)
        ast_factory = create_ts10_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
