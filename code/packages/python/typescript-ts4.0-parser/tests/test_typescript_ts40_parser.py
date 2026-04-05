r"""Tests for the TypeScript 4.0 (2020) Parser.

TypeScript 4.0 introduced:
- Variadic tuple types: ``[...T, ...U]`` for generic tuple concatenation
- Labeled tuple elements: ``[start: number, end: number]``
- Template literal types: ``\`Hello, ${string}!\```
- Short-circuit assignment operators: ``&&=``, ``||=``, ``??=``
- Catch variable narrowing to ``unknown`` with ``useUnknownInCatchVariables``

Test strategy:
- Verify the parser produces ``program`` as the root node
- Verify TypeScript 4.0-specific syntax parses without error
- Verify TS 3.0 features (``unknown`` type) still work
- Verify TypeScript general constructs produce expected AST structure
- Verify the factory function and direct parse produce matching root nodes
"""

from __future__ import annotations

from lang_parser import ASTNode

from typescript_ts40_parser import create_ts40_parser, parse_ts40

# ============================================================================
# Helpers
# ============================================================================


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all AST nodes with the given rule name.

    Performs a depth-first traversal of the AST tree, collecting all nodes
    whose ``rule_name`` matches the requested name.

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
        ast = parse_ts40("")
        assert ast.rule_name == "program"

    def test_empty_program_does_not_raise(self) -> None:
        """Parsing empty input should not raise an exception."""
        ast = parse_ts40("")
        assert ast is not None


# ============================================================================
# Test: Basic Variable Declarations
# ============================================================================


class TestVariableDeclarations:
    """var, let, and const declarations are the most basic TypeScript statements."""

    def test_var_declaration(self) -> None:
        ast = parse_ts40("var x = 1;")
        assert ast.rule_name == "program"

    def test_const_declaration(self) -> None:
        ast = parse_ts40("const x = 1;")
        assert ast.rule_name == "program"

    def test_let_declaration(self) -> None:
        ast = parse_ts40("let x = 1;")
        assert ast.rule_name == "program"

    def test_typed_const_declaration(self) -> None:
        """Type annotations are TypeScript-specific syntax."""
        ast = parse_ts40("const x: number = 1;")
        assert ast.rule_name == "program"

    def test_multiple_declarations(self) -> None:
        ast = parse_ts40("const x = 1; const y = 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript 4.0 — labeled tuple elements (NEW)
# ============================================================================


class TestLabeledTupleElements:
    """TypeScript 4.0 labeled tuple elements: ``[start: number, end: number]``.

    Labels are documentation-only — they do not change the type structure.
    The parser must accept them in type positions without error.
    """

    def test_labeled_tuple_type_declaration(self) -> None:
        """``type Range = [start: number, end: number];`` must parse."""
        ast = parse_ts40("type Range = [start: number, end: number];")
        assert ast.rule_name == "program"

    def test_labeled_optional_tuple_element(self) -> None:
        """Optional labeled elements use ``?`` after the label."""
        ast = parse_ts40("type T = [required: string, optional?: number];")
        assert ast.rule_name == "program"

    def test_labeled_rest_tuple_element(self) -> None:
        """Rest element with a label: ``[first: string, ...rest: number[]]``."""
        ast = parse_ts40("type T = [first: string, ...rest: number[]];")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript 4.0 — variadic tuple types (NEW)
# ============================================================================


class TestVariadicTupleTypes:
    """TypeScript 4.0 variadic tuples: generic spreads in tuple positions.

    The key pattern is
    ``type Concat<T extends unknown[], U extends unknown[]> = [...T, ...U]``.
    This requires the parser to handle generic type parameters that are
    themselves array types, spread inside tuple type positions.
    """

    def test_variadic_tuple_type(self) -> None:
        ast = parse_ts40("type T = [...string[]];")
        assert ast.rule_name == "program"

    def test_concat_type_alias(self) -> None:
        """Classic variadic tuple concat pattern from TS 4.0 release notes."""
        ast = parse_ts40("type Concat<T, U> = [...T[], ...U[]];")
        assert ast.rule_name == "program"

    def test_prepend_type(self) -> None:
        """Prepend pattern: add an element to the front of a tuple."""
        ast = parse_ts40("type Prepend<T, U extends unknown[]> = [T, ...U];")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript 4.0 — short-circuit assignment (NEW)
# ============================================================================


class TestShortCircuitAssignment:
    """TypeScript 4.0 (ES2021) short-circuit assignment operators.

    ``a &&= b`` — assign b only if a is truthy
    ``a ||= b`` — assign b only if a is falsy
    ``a ??= b`` — assign b only if a is null or undefined
    """

    def test_logical_and_assign(self) -> None:
        ast = parse_ts40("a &&= b;")
        assert ast.rule_name == "program"

    def test_logical_or_assign(self) -> None:
        ast = parse_ts40("a ||= b;")
        assert ast.rule_name == "program"

    def test_nullish_coalescing_assign(self) -> None:
        ast = parse_ts40("a ??= b;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: TypeScript 3.0 features still work in TS 4.0
# ============================================================================


class TestTS30Compatibility:
    """TypeScript 4.0 is a superset of TypeScript 3.0."""

    def test_unknown_type_annotation(self) -> None:
        ast = parse_ts40("const x: unknown = 42;")
        assert ast.rule_name == "program"

    def test_unknown_as_parameter_type(self) -> None:
        ast = parse_ts40("function foo(x: unknown): void {}")
        assert ast.rule_name == "program"

    def test_rest_in_tuple_type(self) -> None:
        ast = parse_ts40("type T = [string, ...number[]];")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Type Annotations
# ============================================================================


class TestTypeAnnotations:
    """TypeScript type annotation syntax."""

    def test_number_annotation(self) -> None:
        ast = parse_ts40("var n: number = 0;")
        assert ast.rule_name == "program"

    def test_string_annotation(self) -> None:
        ast = parse_ts40("var s: string = 'hi';")
        assert ast.rule_name == "program"

    def test_boolean_annotation(self) -> None:
        ast = parse_ts40("var b: boolean = true;")
        assert ast.rule_name == "program"

    def test_void_return_type(self) -> None:
        ast = parse_ts40("function f(): void {}")
        assert ast.rule_name == "program"

    def test_union_type(self) -> None:
        ast = parse_ts40("var x: string | number;")
        assert ast.rule_name == "program"

    def test_intersection_type(self) -> None:
        ast = parse_ts40("var x: A & B;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Function Declarations
# ============================================================================


class TestFunctionDeclarations:
    """Function declarations are the core of TypeScript programs."""

    def test_simple_function(self) -> None:
        ast = parse_ts40("function foo() {}")
        func_decls = find_nodes(ast, "function_declaration")
        assert len(func_decls) >= 1

    def test_typed_function(self) -> None:
        ast = parse_ts40("function add(a: number, b: number): number { return a + b; }")
        assert ast.rule_name == "program"

    def test_generic_function(self) -> None:
        ast = parse_ts40("function identity<T>(x: T): T { return x; }")
        assert ast.rule_name == "program"

    def test_arrow_function(self) -> None:
        ast = parse_ts40("const f = (x: number) => x * 2;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Classes
# ============================================================================


class TestClasses:
    """TypeScript classes with type annotations."""

    def test_simple_class(self) -> None:
        ast = parse_ts40("class Foo {}")
        assert ast.rule_name == "program"

    def test_class_with_typed_property(self) -> None:
        ast = parse_ts40("class Foo { name: string = ''; }")
        assert ast.rule_name == "program"

    def test_class_extending_class(self) -> None:
        ast = parse_ts40("class B extends A {}")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Control Flow
# ============================================================================


class TestControlFlow:
    """Standard control flow constructs must all parse correctly."""

    def test_if_statement(self) -> None:
        ast = parse_ts40("if (x) {}")
        if_stmts = find_nodes(ast, "if_statement")
        assert len(if_stmts) >= 1

    def test_for_loop(self) -> None:
        ast = parse_ts40("for (var i = 0; i < 10; i++) {}")
        assert ast.rule_name == "program"

    def test_try_catch(self) -> None:
        ast = parse_ts40("try {} catch (e) {}")
        assert ast.rule_name == "program"

    def test_while_loop(self) -> None:
        ast = parse_ts40("while (true) { break; }")
        assert ast.rule_name == "program"


# ============================================================================
# Test: ES2020 features
# ============================================================================


class TestES2020Features:
    """TypeScript 4.0 builds on ES2020 — optional chaining and nullish coalescing."""

    def test_optional_chaining(self) -> None:
        """``?.`` is the optional chaining operator from ES2020."""
        ast = parse_ts40("const n = obj?.name;")
        assert ast.rule_name == "program"

    def test_nullish_coalescing(self) -> None:
        """``??`` is the nullish coalescing operator from ES2020."""
        ast = parse_ts40("const y = x ?? 0;")
        assert ast.rule_name == "program"


# ============================================================================
# Test: Factory Function
# ============================================================================


class TestCreateTS40Parser:
    """The factory function ``create_ts40_parser`` should return a usable parser."""

    def test_creates_parser_with_parse_method(self) -> None:
        parser = create_ts40_parser("const x = 1;")
        assert hasattr(parser, "parse")

    def test_factory_and_direct_produce_same_root(self) -> None:
        """Both paths must produce the same root rule name."""
        source = "type Pair = [first: string, second: number];"
        ast_direct = parse_ts40(source)
        ast_factory = create_ts40_parser(source).parse()
        assert ast_direct.rule_name == ast_factory.rule_name

    def test_factory_with_empty_source(self) -> None:
        parser = create_ts40_parser("")
        ast = parser.parse()
        assert ast.rule_name == "program"
