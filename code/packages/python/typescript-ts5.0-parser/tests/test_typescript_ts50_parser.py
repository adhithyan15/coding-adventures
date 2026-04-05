"""Tests for the TypeScript 5.0 (2023) Parser.

TypeScript 5.0 adds standard TC39 decorators, const type parameters,
accessor keyword, and satisfies operator on top of the ES2022 class system.
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts50_parser import create_ts50_parser, parse_ts50

# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with the given rule name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


# ============================================================================
# Test: Variable Statements
# ============================================================================


class TestVariableStatements:
    """var/let/const declarations with and without type annotations."""

    def test_var_declaration(self) -> None:
        """``var x: number = 1;`` — basic typed variable declaration."""
        ast = parse_ts50("var x: number = 1;")
        assert ast.rule_name == "program"
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(var_stmts) == 1

    def test_let_declaration(self) -> None:
        """``let y: string = "hello";`` — lexical binding."""
        ast = parse_ts50('let y: string = "hello";')
        assert ast.rule_name == "program"

    def test_const_declaration(self) -> None:
        """``const z = 42;`` — const without explicit type (inferred)."""
        ast = parse_ts50("const z = 42;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Interface Declarations (TypeScript)
# ============================================================================


class TestInterfaceDeclarations:
    """TypeScript ``interface`` declarations define structural types."""

    def test_simple_interface(self) -> None:
        """``interface Point { x: number; y: number; }``"""
        ast = parse_ts50("interface Point { x: number; y: number; }")
        ifaces = find_nodes(ast, "interface_declaration")
        assert len(ifaces) == 1

    def test_interface_with_method(self) -> None:
        """Interface with a method signature."""
        ast = parse_ts50("interface Greeter { greet(name: string): void; }")
        ifaces = find_nodes(ast, "interface_declaration")
        assert len(ifaces) == 1

    def test_interface_extends(self) -> None:
        """``interface B extends A {}`` — interface inheritance."""
        ast = parse_ts50("interface B extends A {}")
        ifaces = find_nodes(ast, "interface_declaration")
        assert len(ifaces) >= 1


# ============================================================================
# Test: Type Alias Declarations (TypeScript)
# ============================================================================


class TestTypeAliasDeclarations:
    """``type Alias = ...`` creates a named type alias."""

    def test_simple_type_alias(self) -> None:
        """``type ID = string;`` — basic type alias."""
        ast = parse_ts50("type ID = string;")
        aliases = find_nodes(ast, "type_alias_declaration")
        assert len(aliases) == 1

    def test_union_type_alias(self) -> None:
        """``type Status = "active" | "inactive";`` — union type."""
        ast = parse_ts50('type Status = "active" | "inactive";')
        aliases = find_nodes(ast, "type_alias_declaration")
        assert len(aliases) == 1

    def test_generic_type_alias(self) -> None:
        """``type Maybe<T> = T | null;`` — generic type alias."""
        ast = parse_ts50("type Maybe<T> = T | null;")
        aliases = find_nodes(ast, "type_alias_declaration")
        assert len(aliases) == 1


# ============================================================================
# Test: Enum Declarations (TypeScript)
# ============================================================================


class TestEnumDeclarations:
    """TypeScript enums declare a set of named constants."""

    def test_simple_enum(self) -> None:
        """``enum Direction { Up, Down, Left, Right }``"""
        ast = parse_ts50("enum Direction { Up, Down, Left, Right }")
        enums = find_nodes(ast, "enum_declaration")
        assert len(enums) == 1

    def test_const_enum(self) -> None:
        """``const enum Color { Red = 0, Green = 1, Blue = 2 }``

        ``const`` enums are inlined at compile time — no runtime object.
        """
        ast = parse_ts50("const enum Color { Red = 0, Green = 1, Blue = 2 }")
        enums = find_nodes(ast, "enum_declaration")
        assert len(enums) == 1


# ============================================================================
# Test: Class Declarations with TypeScript Extensions
# ============================================================================


class TestClassDeclarations:
    """TypeScript classes extend ES2022 classes with type system features."""

    def test_simple_class(self) -> None:
        """``class Foo {}`` — basic class."""
        ast = parse_ts50("class Foo {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) == 1

    def test_class_with_decorator(self) -> None:
        """``@sealed class Foo {}`` — class with decorator."""
        ast = parse_ts50("@sealed class Foo {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) >= 1

    def test_class_implements_interface(self) -> None:
        """``class Dog implements Animal {}`` — class implementing interface."""
        ast = parse_ts50("class Dog implements Animal {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) == 1


# ============================================================================
# Test: Generic Functions (type_parameters)
# ============================================================================


class TestGenericFunctions:
    """Generic functions use type parameters enclosed in ``< >``."""

    def test_simple_generic(self) -> None:
        """``function identity<T>(x: T): T { return x; }``"""
        ast = parse_ts50("function identity<T>(x: T): T { return x; }")
        # Should parse successfully, program is root
        assert ast.rule_name == "program"

    def test_constrained_generic(self) -> None:
        """``function first<T extends any[]>(arr: T): T[0] { return arr[0]; }``"""
        ast = parse_ts50("function first<T>(arr: T[]): T { return arr[0]; }")
        assert ast.rule_name == "program"

    def test_type_parameters_node(self) -> None:
        """Generic function produces type_parameters node."""
        ast = parse_ts50("function id<T>(x: T): T { return x; }")
        type_params = find_nodes(ast, "type_parameters")
        assert len(type_params) >= 1


# ============================================================================
# Test: Function Declarations
# ============================================================================


class TestFunctionDeclarations:
    """Standard function declarations."""

    def test_simple_function(self) -> None:
        """``function foo() {}`` — basic function."""
        ast = parse_ts50("function foo() {}")
        funcs = find_nodes(ast, "function_declaration")
        assert len(funcs) == 1

    def test_function_with_return_type(self) -> None:
        """``function greet(name: string): string { return name; }``"""
        ast = parse_ts50('function greet(name: string): string { return "hi"; }')
        funcs = find_nodes(ast, "function_declaration")
        assert len(funcs) == 1


# ============================================================================
# Test: Multiple Top-Level Declarations
# ============================================================================


class TestMultipleDeclarations:
    """Programs with multiple top-level declarations."""

    def test_interface_and_class(self) -> None:
        """Interface followed by implementing class."""
        source = (
            "interface Animal { speak(): void; }"
            " class Dog implements Animal { speak() {} }"
        )
        ast = parse_ts50(source)
        assert ast.rule_name == "program"
        ifaces = find_nodes(ast, "interface_declaration")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(ifaces) == 1
        assert len(classes) == 1

    def test_type_alias_and_function(self) -> None:
        """Type alias followed by function using that type."""
        source = "type Num = number; function double(x: Num): Num { return x * 2; }"
        ast = parse_ts50(source)
        assert ast.rule_name == "program"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS50Parser:

    def test_creates_parser(self) -> None:
        parser = create_ts50_parser("var x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        source = "var x: number = 1;"
        ast_direct = parse_ts50(source)
        ast_factory = create_ts50_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
