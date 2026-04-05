"""Tests for the TypeScript 3.0 (2018) Parser.

TypeScript 3.0 introduced:
- The ``unknown`` top type as a safer alternative to ``any``
- Rest elements in tuple types: ``[string, ...number[]]``
- Project references for large monorepos

Test strategy:
- Verify the parser produces ``program`` as the root node
- Verify TypeScript 3.0-specific syntax parses without error
- Verify TypeScript general constructs produce expected AST structure
- Verify the factory function and direct parse produce matching root nodes
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts30_parser import create_ts30_parser, parse_ts30

# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with the given rule name.

    This is the standard tree-walking helper used throughout the parser test
    suites. It performs a depth-first traversal of the AST, collecting all
    nodes whose ``rule_name`` matches the requested name.

    Args:
        node: The root node to start searching from.
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
# Test: Empty Program
# ============================================================================


class TestEmptyProgram:
    """An empty source string must still produce a valid ``program`` root."""

    def test_empty_program_root_is_program(self) -> None:
        ast = parse_ts30("")
        assert ast.rule_name == "program"

    def test_empty_program_has_no_children_errors(self) -> None:
        """Parsing empty input should not raise an exception."""
        ast = parse_ts30("")
        assert ast is not None


# ============================================================================
# Test: Basic Variable Declarations
# ============================================================================


class TestVariableDeclarations:
    """var, let, and const declarations are the most basic TypeScript statements."""

    def test_var_declaration(self) -> None:
        ast = parse_ts30("var x = 1;")
        assert ast.rule_name == "program"

    def test_const_declaration(self) -> None:
        ast = parse_ts30("const x = 1;")
        assert ast.rule_name == "program"

    def test_let_declaration(self) -> None:
        ast = parse_ts30("let x = 1;")
        assert ast.rule_name == "program"

    def test_typed_var_declaration(self) -> None:
        """Type annotations are TypeScript-specific syntax."""
        ast = parse_ts30("var x: number = 1;")
        assert ast.rule_name == "program"

    def test_multiple_declarations(self) -> None:
        ast = parse_ts30("var x = 1; var y = 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript 3.0 — unknown type (NEW)
# ============================================================================


class TestUnknownType:
    """TypeScript 3.0 introduced ``unknown`` as the type-safe top type.

    Unlike ``any``, you cannot use an ``unknown`` value without narrowing.
    At the parser level, ``unknown`` appears as an identifier in type positions.
    """

    def test_unknown_type_annotation(self) -> None:
        """``const x: unknown = 42;`` must parse without error."""
        ast = parse_ts30("const x: unknown = 42;")
        assert ast.rule_name == "program"

    def test_unknown_as_parameter_type(self) -> None:
        """Function parameters can have the ``unknown`` type."""
        ast = parse_ts30("function foo(x: unknown): void {}")
        assert ast.rule_name == "program"

    def test_unknown_in_function_return_type(self) -> None:
        ast = parse_ts30("function getValue(): unknown { return 42; }")
        assert ast.rule_name == "program"

    def test_unknown_type_narrowing_pattern(self) -> None:
        """Type narrowing with ``unknown`` is the canonical TS 3.0 pattern."""
        source = (
            "function handle(val: unknown) { if (typeof val === 'string')"
            " { var s = val; } }"
        )
        ast = parse_ts30(source)
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript type annotations
# ============================================================================


class TestTypeAnnotations:
    """TypeScript-specific type annotation syntax."""

    def test_number_annotation(self) -> None:
        ast = parse_ts30("var n: number = 0;")
        assert ast.rule_name == "program"

    def test_string_annotation(self) -> None:
        ast = parse_ts30("var s: string = 'hi';")
        assert ast.rule_name == "program"

    def test_boolean_annotation(self) -> None:
        ast = parse_ts30("var b: boolean = true;")
        assert ast.rule_name == "program"

    def test_any_annotation(self) -> None:
        ast = parse_ts30("var x: any = null;")
        assert ast.rule_name == "program"

    def test_void_return_type(self) -> None:
        ast = parse_ts30("function f(): void {}")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Function Declarations
# ============================================================================


class TestFunctionDeclarations:
    """Function declarations are the core of TypeScript programs."""

    def test_simple_function(self) -> None:
        ast = parse_ts30("function foo() {}")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) >= 1

    def test_typed_function(self) -> None:
        ast = parse_ts30("function add(a: number, b: number): number { return a + b; }")
        assert ast.rule_name == "program"

    def test_generic_function(self) -> None:
        """Generic functions use type parameters ``<T>``."""
        ast = parse_ts30("function identity<T>(x: T): T { return x; }")
        assert ast.rule_name == "program"

    def test_arrow_function(self) -> None:
        ast = parse_ts30("const f = (x: number) => x * 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Interfaces
# ============================================================================


class TestInterfaces:
    """TypeScript interfaces describe the shape of objects."""

    def test_simple_interface(self) -> None:
        ast = parse_ts30("interface Foo { bar: string; }")
        assert ast.rule_name == "program"

    def test_interface_with_method(self) -> None:
        ast = parse_ts30("interface Animal { speak(): void; }")
        assert ast.rule_name == "program"

    def test_interface_extending_interface(self) -> None:
        ast = parse_ts30("interface B extends A { extra: number; }")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Classes
# ============================================================================


class TestClasses:
    """TypeScript classes with type annotations and access modifiers."""

    def test_simple_class(self) -> None:
        ast = parse_ts30("class Foo {}")
        assert ast.rule_name == "program"

    def test_class_with_constructor(self) -> None:
        ast = parse_ts30("class Foo { constructor(public x: number) {} }")
        assert ast.rule_name == "program"

    def test_class_extending_class(self) -> None:
        ast = parse_ts30("class B extends A {}")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Control flow
# ============================================================================


class TestControlFlow:
    """Standard control flow constructs must all parse correctly."""

    def test_if_statement(self) -> None:
        ast = parse_ts30("if (x) {}")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) >= 1

    def test_if_else(self) -> None:
        ast = parse_ts30("if (x) {} else {}")
        assert ast.rule_name == "program"

    def test_for_loop(self) -> None:
        ast = parse_ts30("for (var i = 0; i < 10; i++) {}")
        assert ast.rule_name == "program"

    def test_while_loop(self) -> None:
        ast = parse_ts30("while (true) { break; }")
        assert ast.rule_name == "program"

    def test_try_catch(self) -> None:
        ast = parse_ts30("try {} catch (e) {}")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS30Parser:
    """The factory function ``create_ts30_parser`` should return a usable parser."""

    def test_creates_parser_with_parse_method(self) -> None:
        parser = create_ts30_parser("const x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_and_direct_produce_same_root(self) -> None:
        """Both paths must produce the same root rule name."""
        source = "const x: unknown = 42;"
        ast_direct = parse_ts30(source)
        ast_factory = create_ts30_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name

    def test_factory_with_empty_source(self) -> None:
        parser = create_ts30_parser("")
        ast = parser.parse()
        assert ast.rule_name == "program"
