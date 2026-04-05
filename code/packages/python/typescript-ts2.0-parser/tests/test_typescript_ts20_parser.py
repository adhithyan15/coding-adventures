"""Tests for the TypeScript 2.0 (September 2016) Parser.

TypeScript 2.0 extended TS 1.0 with ECMAScript 2015 syntax (arrow functions,
classes, modules, destructuring) and new type system features (never type,
non-nullable types, mapped types).

The root of every AST is a ``program`` node.
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts20_parser import create_ts20_parser, parse_ts20

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

    def test_simple_program(self) -> None:
        ast = parse_ts20("const x: number = 1;")
        assert ast.rule_name == "program"

    def test_empty_program(self) -> None:
        ast = parse_ts20("")
        assert ast.rule_name == "program"

    def test_never_type_program(self) -> None:
        ast = parse_ts20("declare function fail(): never;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Arrow Functions (ES2015)
# ============================================================================


class TestArrowFunctions:
    """Arrow functions are new grammar rules in the ES2015 baseline.

    Example: ``const double = (x: number): number => x * 2;``

    The grammar rule is ``arrow_function`` or ``arrow_function_expression``.
    """

    def test_typed_arrow_function(self) -> None:
        ast = parse_ts20("const double = (x: number): number => x * 2;")
        arrow_nodes = find_nodes(ast, "arrow_function")
        assert len(arrow_nodes) >= 1

    def test_simple_arrow_function(self) -> None:
        ast = parse_ts20("const add = (a, b) => a + b;")
        arrow_nodes = find_nodes(ast, "arrow_function")
        assert len(arrow_nodes) >= 1

    def test_expression_body_arrow(self) -> None:
        ast = parse_ts20("const inc = x => x + 1;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: ES2015 Classes
# ============================================================================


class TestES2015Classes:
    """ES2015 class declarations with TypeScript additions.

    Example: ``class Animal extends Base implements IAnimal { }``

    In TS 2.0 grammar the rule may be ``ts_class_declaration`` (for classes
    with TypeScript-specific extensions) or ``class_declaration``.
    """

    def test_class_with_implements(self) -> None:
        ast = parse_ts20("class Foo implements IFoo { }")
        class_nodes = (
            find_nodes(ast, "ts_class_declaration")
            or find_nodes(ast, "class_declaration")
        )
        assert len(class_nodes) >= 1

    def test_class_extends_and_implements(self) -> None:
        ast = parse_ts20("class Dog extends Animal implements IAnimal { }")
        class_nodes = (
            find_nodes(ast, "ts_class_declaration")
            or find_nodes(ast, "class_declaration")
        )
        assert len(class_nodes) >= 1

    def test_class_with_typed_members(self) -> None:
        ast = parse_ts20("class Point { x: number; y: number; }")
        class_nodes = (
            find_nodes(ast, "ts_class_declaration")
            or find_nodes(ast, "class_declaration")
        )
        assert len(class_nodes) >= 1


# ============================================================================
# Test: ES2015 Modules
# ============================================================================


class TestES2015Modules:
    """ES2015 module import and export declarations.

    Example: ``import { Foo } from "./foo";``

    The grammar rule is ``import_declaration``.
    """

    def test_named_import(self) -> None:
        ast = parse_ts20('import { Foo } from "./foo";')
        import_nodes = find_nodes(ast, "import_declaration")
        assert len(import_nodes) >= 1

    def test_default_import(self) -> None:
        ast = parse_ts20('import Foo from "./foo";')
        import_nodes = find_nodes(ast, "import_declaration")
        assert len(import_nodes) >= 1

    def test_export_declaration(self) -> None:
        ast = parse_ts20("export function foo() {}")
        export_nodes = find_nodes(ast, "export_declaration")
        assert len(export_nodes) >= 1


# ============================================================================
# Test: never Type in Type Positions
# ============================================================================


class TestNeverType:
    """The ``never`` type appears in type positions.

    Example: ``function fail(msg: string): never { throw new Error(msg); }``

    The ``never`` token is a NAME in the lexer; the parser resolves it as
    a ``never_type`` node in type position.
    """

    def test_function_returning_never(self) -> None:
        ast = parse_ts20("declare function fail(msg: string): never;")
        ambient_nodes = find_nodes(ast, "ambient_declaration")
        assert len(ambient_nodes) >= 1

    def test_never_program_root(self) -> None:
        ast = parse_ts20("type Bottom = never;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript Interfaces (inherited from TS 1.0)
# ============================================================================


class TestInterfaceDeclaration:
    """Interface declarations inherited from TS 1.0."""

    def test_basic_interface(self) -> None:
        ast = parse_ts20("interface Foo { name: string; }")
        iface_nodes = find_nodes(ast, "interface_declaration")
        assert len(iface_nodes) >= 1

    def test_interface_extending(self) -> None:
        ast = parse_ts20("interface Bar extends Foo { age: number; }")
        iface_nodes = find_nodes(ast, "interface_declaration")
        assert len(iface_nodes) >= 1


# ============================================================================
# Test: Type Aliases (inherited from TS 1.0)
# ============================================================================


class TestTypeAliases:
    """Type alias declarations with TS 2.0 types."""

    def test_union_with_never(self) -> None:
        ast = parse_ts20("type Result = string | never;")
        alias_nodes = find_nodes(ast, "type_alias_declaration")
        assert len(alias_nodes) >= 1

    def test_nullable_type_alias(self) -> None:
        ast = parse_ts20("type MaybeString = string | null | undefined;")
        alias_nodes = find_nodes(ast, "type_alias_declaration")
        assert len(alias_nodes) >= 1


# ============================================================================
# Test: Destructuring
# ============================================================================


class TestDestructuring:
    """ES2015 destructuring in variable declarations.

    Example: ``const { x, y } = obj;``
    """

    def test_object_destructuring(self) -> None:
        ast = parse_ts20("const { x, y } = obj;")
        assert ast.rule_name == "program"

    def test_array_destructuring(self) -> None:
        ast = parse_ts20("const [first, second] = arr;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TS 1.0 Compatibility
# ============================================================================


class TestTS10Compatibility:
    """All TS 1.0 constructs must parse correctly in TS 2.0."""

    def test_enum_declaration(self) -> None:
        ast = parse_ts20("enum Color { Red, Green, Blue }")
        enum_nodes = find_nodes(ast, "enum_declaration")
        assert len(enum_nodes) >= 1

    def test_namespace_declaration(self) -> None:
        ast = parse_ts20("namespace MyNS { }")
        ns_nodes = find_nodes(ast, "namespace_declaration")
        assert len(ns_nodes) >= 1

    def test_ambient_declaration(self) -> None:
        ast = parse_ts20("declare var x: number;")
        ambient_nodes = find_nodes(ast, "ambient_declaration")
        assert len(ambient_nodes) >= 1


# ============================================================================
# Test: ES5 Compatibility
# ============================================================================


class TestES5Compatibility:
    """All ES5 constructs must parse correctly in TS 2.0."""

    def test_var_declaration(self) -> None:
        ast = parse_ts20("var x = 1;")
        assert ast.rule_name == "program"

    def test_if_else(self) -> None:
        ast = parse_ts20("if (x) { } else { }")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) >= 1

    def test_try_catch(self) -> None:
        ast = parse_ts20("try { } catch (e) { }")
        try_stmts = find_nodes(ast, "try_statement")
        assert len(try_stmts) >= 1

    def test_debugger_statement(self) -> None:
        ast = parse_ts20("debugger;")
        dbg_stmts = find_nodes(ast, "debugger_statement")
        assert len(dbg_stmts) >= 1


# ============================================================================
# Test: Multiple Statements
# ============================================================================


class TestMultipleStatements:
    """Multiple TypeScript 2.0 declarations in one source file."""

    def test_import_and_interface(self) -> None:
        ast = parse_ts20(
            'import { Foo } from "./foo";\n'
            "interface Bar extends Foo { age: number; }"
        )
        import_nodes = find_nodes(ast, "import_declaration")
        iface_nodes = find_nodes(ast, "interface_declaration")
        assert len(import_nodes) >= 1
        assert len(iface_nodes) >= 1

    def test_class_and_interface(self) -> None:
        ast = parse_ts20(
            "interface IPoint { x: number; y: number; }\n"
            "class Point implements IPoint { x: number = 0; y: number = 0; }"
        )
        iface_nodes = find_nodes(ast, "interface_declaration")
        class_nodes = (
            find_nodes(ast, "ts_class_declaration")
            or find_nodes(ast, "class_declaration")
        )
        assert len(iface_nodes) >= 1
        assert len(class_nodes) >= 1


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS20Parser:
    """Tests for the ``create_ts20_parser`` factory function."""

    def test_creates_parser(self) -> None:
        """Factory returns an object with a parse method."""
        parser = create_ts20_parser("const x: number = 1;")
        assert hasattr(parser, "parse")

    def test_factory_produces_same_result(self) -> None:
        """Factory result matches direct parse_ts20 call."""
        source = "const x: number = 1;"
        ast_direct = parse_ts20(source)
        ast_factory = create_ts20_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name
