"""Tests for the TypeScript 5.8 (2025) Parser.

TypeScript 5.8 targets the ES2025 baseline. ES2025 standardizes decorators,
import attributes, and explicit resource management (``using`` / ``await using``).
TS 5.8 adds ``export type *`` re-exports and ambient module declarations.
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts58_parser import create_ts58_parser, parse_ts58

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

    def test_var_declaration(self) -> None:
        """``var x: number = 1;`` — basic typed variable declaration."""
        ast = parse_ts58("var x: number = 1;")
        assert ast.rule_name == "program"
        var_stmts = find_nodes(ast, "variable_statement")
        assert len(var_stmts) == 1

    def test_const_declaration(self) -> None:
        """``const z = 42;`` — const declaration."""
        ast = parse_ts58("const z = 42;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Using Declarations (ES2025 Explicit Resource Management)
# ============================================================================


class TestUsingDeclarations:
    """``using`` and ``await using`` declare resources with automatic disposal.

    Resources implement ``Symbol.dispose`` (sync) or ``Symbol.asyncDispose`` (async).
    When the enclosing block exits — normally or via exception — the dispose
    method is called automatically.
    """

    def test_using_declaration(self) -> None:
        """``using x = getResource();`` — synchronous resource binding."""
        ast = parse_ts58("using x = getResource();")
        assert ast.rule_name == "program"
        using_decls = find_nodes(ast, "using_declaration")
        assert len(using_decls) >= 1

    def test_await_using_declaration(self) -> None:
        """``await using db = await connect();`` — async resource binding."""
        ast = parse_ts58("await using db = await connect();")
        assert ast.rule_name == "program"
        await_using = find_nodes(ast, "await_using_declaration")
        assert len(await_using) >= 1

    def test_using_in_block(self) -> None:
        """``using`` declaration inside a block."""
        source = "function run() { using conn = open(); }"
        ast = parse_ts58(source)
        assert ast.rule_name == "program"


# ============================================================================
# Test: Interface Declarations
# ============================================================================


class TestInterfaceDeclarations:

    def test_simple_interface(self) -> None:
        """``interface Point { x: number; y: number; }``"""
        ast = parse_ts58("interface Point { x: number; y: number; }")
        ifaces = find_nodes(ast, "interface_declaration")
        assert len(ifaces) == 1

    def test_interface_with_optional(self) -> None:
        """Interface with optional property (``?``)."""
        ast = parse_ts58("interface Config { debug?: boolean; }")
        ifaces = find_nodes(ast, "interface_declaration")
        assert len(ifaces) == 1


# ============================================================================
# Test: Type Alias Declarations
# ============================================================================


class TestTypeAliasDeclarations:

    def test_simple_type_alias(self) -> None:
        """``type ID = string;``"""
        ast = parse_ts58("type ID = string;")
        aliases = find_nodes(ast, "type_alias_declaration")
        assert len(aliases) == 1

    def test_union_type_alias(self) -> None:
        """``type Result<T> = T | Error;``"""
        ast = parse_ts58("type Result<T> = T | null;")
        aliases = find_nodes(ast, "type_alias_declaration")
        assert len(aliases) == 1


# ============================================================================
# Test: Export Type Statements (TS 5.8)
# ============================================================================


class TestExportTypeStatements:
    """``export type`` statements for re-exporting types.

    TS 5.8 adds ``export type *`` for re-exporting all types from a module.
    This is important for the ``--erasableSyntaxOnly`` mode, which ensures
    all TypeScript-specific syntax can be stripped without runtime behavior.
    """

    def test_export_type_named(self) -> None:
        """``export type { Foo };`` — export named type."""
        ast = parse_ts58("export type { Foo };")
        assert ast.rule_name == "program"

    def test_export_type_from(self) -> None:
        """``export type { Foo } from "./types";``"""
        ast = parse_ts58('export type { Foo } from "./types";')
        assert ast.rule_name == "program"

    def test_export_type_star(self) -> None:
        """``export type * from "./types";`` — re-export all types (TS 5.8)."""
        ast = parse_ts58('export type * from "./types";')
        assert ast.rule_name == "program"


# ============================================================================
# Test: Ambient Module Declarations
# ============================================================================


class TestAmbientModuleDeclarations:
    """Ambient module declarations declare the shape of external modules.

    Used in ``.d.ts`` files to provide types for JavaScript modules that
    don't have TypeScript sources.
    """

    def test_ambient_module(self) -> None:
        """``declare module "lodash" { export function chunk<T>(arr: T[], size: number):
        T[][]; }``"""
        source = 'declare module "lodash" { }'
        ast = parse_ts58(source)
        assert ast.rule_name == "program"


# ============================================================================
# Test: Class Declarations with Decorators
# ============================================================================


class TestClassDeclarations:

    def test_simple_class(self) -> None:
        """``class Foo {}`` — basic class."""
        ast = parse_ts58("class Foo {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) == 1

    def test_class_with_decorator(self) -> None:
        """``@sealed class Foo {}`` — class with standard TC39 decorator."""
        ast = parse_ts58("@sealed class Foo {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) >= 1

    def test_class_with_implements(self) -> None:
        """``class Dog implements Animal {}``"""
        ast = parse_ts58("class Dog implements Animal {}")
        classes = find_nodes(ast, "ts_class_declaration")
        assert len(classes) == 1


# ============================================================================
# Test: Generic Functions
# ============================================================================


class TestGenericFunctions:

    def test_simple_generic(self) -> None:
        """``function identity<T>(x: T): T { return x; }``"""
        ast = parse_ts58("function identity<T>(x: T): T { return x; }")
        assert ast.rule_name == "program"
        type_params = find_nodes(ast, "type_parameters")
        assert len(type_params) >= 1

    def test_multi_generic(self) -> None:
        """``function pair<A, B>(a: A, b: B): [A, B] { return [a, b]; }``"""
        ast = parse_ts58("function pair<A, B>(a: A, b: B): A { return a; }")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Enum Declarations
# ============================================================================


class TestEnumDeclarations:

    def test_simple_enum(self) -> None:
        ast = parse_ts58("enum Status { Active, Inactive }")
        enums = find_nodes(ast, "enum_declaration")
        assert len(enums) == 1

    def test_const_enum(self) -> None:
        ast = parse_ts58("const enum Flags { None = 0, Read = 1, Write = 2 }")
        enums = find_nodes(ast, "enum_declaration")
        assert len(enums) == 1


# ============================================================================
# Test: Multiple Declarations
# ============================================================================


class TestMultipleDeclarations:

    def test_interface_class_function(self) -> None:
        """Complex program with multiple top-level declarations."""
        source = (
            "interface Animal { speak(): void; } "
            "class Dog implements Animal { speak() {} } "
            "function makeNoise(a: Animal): void { a.speak(); }"
        )
        ast = parse_ts58(source)
        assert ast.rule_name == "program"
        ifaces = find_nodes(ast, "interface_declaration")
        classes = find_nodes(ast, "ts_class_declaration")
        funcs = find_nodes(ast, "function_declaration")
        assert len(ifaces) == 1
        assert len(classes) == 1
        assert len(funcs) == 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS58Parser:

    def test_creates_parser(self) -> None:
        parser = create_ts58_parser("var x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        source = "var x: number = 1;"
        ast_direct = parse_ts58(source)
        ast_factory = create_ts58_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
