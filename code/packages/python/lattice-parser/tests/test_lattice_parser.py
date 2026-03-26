"""Tests for the Lattice parser.

These tests verify that the Lattice parser correctly handles all Lattice
constructs (variables, mixins, control flow, functions, modules) and
plain CSS passthrough. The parser produces generic ``ASTNode`` trees
from the ``GrammarParser``.

Test Strategy
-------------

We test by parsing source strings and inspecting the resulting AST:

- ``rule_name`` tells us which grammar rule matched.
- ``children`` contains sub-nodes (``ASTNode``) and raw tokens (``Token``).
- Leaf nodes wrap a single token (``node.is_leaf`` / ``node.token``).

Helper functions extract rule names and token values to make assertions
readable without deep tree traversal in every test.
"""

from __future__ import annotations

from lattice_parser import __version__, create_lattice_parser, parse_lattice


# ---------------------------------------------------------------------------
# Helper Functions
# ---------------------------------------------------------------------------


def _child_rules(node: object) -> list[str]:
    """Get the rule_name of each ASTNode child (skip tokens)."""
    return [
        c.rule_name
        for c in node.children  # type: ignore[attr-defined]
        if hasattr(c, "rule_name")
    ]


def _child_token_values(node: object) -> list[str]:
    """Get the value of each Token child (skip ASTNodes)."""
    return [
        c.value
        for c in node.children  # type: ignore[attr-defined]
        if not hasattr(c, "rule_name")
    ]


def _token_type(token: object) -> str:
    """Get the string name of a token's type."""
    t = token.type  # type: ignore[attr-defined]
    if isinstance(t, str):
        return t
    return t.name


def _first_rule(node: object) -> object:
    """Get the first child that is an ASTNode."""
    for c in node.children:  # type: ignore[attr-defined]
        if hasattr(c, "rule_name"):
            return c
    raise ValueError("No ASTNode child found")


def _find_rule(node: object, name: str) -> object:
    """Recursively find the first descendant with the given rule_name."""
    if hasattr(node, "rule_name") and node.rule_name == name:  # type: ignore[attr-defined]
        return node
    if hasattr(node, "children"):
        for c in node.children:  # type: ignore[attr-defined]
            try:
                return _find_rule(c, name)
            except ValueError:
                continue
    raise ValueError(f"No node with rule_name={name!r} found")


def _collect_rules(node: object, name: str) -> list[object]:
    """Recursively collect all descendants with the given rule_name."""
    results: list[object] = []
    if hasattr(node, "rule_name") and node.rule_name == name:  # type: ignore[attr-defined]
        results.append(node)
    if hasattr(node, "children"):
        for c in node.children:  # type: ignore[attr-defined]
            results.extend(_collect_rules(c, name))
    return results


# ===========================================================================
# Factory Function Tests
# ===========================================================================


class TestFactory:
    """Test that factory functions return correct types."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"

    def test_create_lattice_parser_returns_parser(self) -> None:
        """create_lattice_parser returns a GrammarParser with a parse method."""
        parser = create_lattice_parser("h1 { color: red; }")
        assert hasattr(parser, "parse")

    def test_parse_lattice_returns_ast_node(self) -> None:
        """parse_lattice returns an ASTNode."""
        ast = parse_lattice("h1 { color: red; }")
        assert hasattr(ast, "rule_name")
        assert hasattr(ast, "children")

    def test_root_is_stylesheet(self) -> None:
        """The root node has rule_name='stylesheet'."""
        ast = parse_lattice("h1 { color: red; }")
        assert ast.rule_name == "stylesheet"

    def test_empty_source(self) -> None:
        """Empty source produces a stylesheet with no children rules."""
        ast = parse_lattice("")
        assert ast.rule_name == "stylesheet"
        # stylesheet = { rule } ; — zero repetitions is valid
        assert len(ast.children) == 0


# ===========================================================================
# Variable Declaration Tests
# ===========================================================================


class TestVariableDeclaration:
    """Test parsing of Lattice variable declarations."""

    def test_simple_variable(self) -> None:
        """$color: red; parses as a variable_declaration."""
        ast = parse_lattice("$color: red;")
        var_decl = _find_rule(ast, "variable_declaration")
        assert var_decl is not None

    def test_variable_with_hash_value(self) -> None:
        """$color: #4a90d9; parses correctly."""
        ast = parse_lattice("$color: #4a90d9;")
        var_decl = _find_rule(ast, "variable_declaration")
        # The variable token should be $color
        tokens = _child_token_values(var_decl)
        assert "$color" in tokens

    def test_variable_with_dimension_value(self) -> None:
        """$size: 16px; parses correctly."""
        ast = parse_lattice("$size: 16px;")
        var_decl = _find_rule(ast, "variable_declaration")
        value_list = _find_rule(var_decl, "value_list")
        assert value_list is not None

    def test_variable_with_multi_value(self) -> None:
        """$font: Helvetica, sans-serif; parses correctly.

        The value_list rule is: value { value }. Commas are valid values
        (value = ... | COMMA | ...), so the whole thing is one value_list.
        """
        ast = parse_lattice("$font: Helvetica, sans-serif;")
        var_decl = _find_rule(ast, "variable_declaration")
        value_list = _find_rule(var_decl, "value_list")
        assert value_list is not None

    def test_multiple_variables(self) -> None:
        """Multiple variable declarations parse as separate rules."""
        ast = parse_lattice("$a: red; $b: blue;")
        var_decls = _collect_rules(ast, "variable_declaration")
        assert len(var_decls) == 2


# ===========================================================================
# Mixin Definition Tests
# ===========================================================================


class TestMixinDefinition:
    """Test parsing of @mixin definitions."""

    def test_mixin_no_params(self) -> None:
        """@mixin clearfix() {} parses as mixin_definition.

        Note: clearfix() tokenizes as FUNCTION token "clearfix(".
        """
        ast = parse_lattice("@mixin clearfix() {}")
        mixin = _find_rule(ast, "mixin_definition")
        assert mixin is not None

    def test_mixin_with_params(self) -> None:
        """@mixin button($bg, $fg) {} parses with mixin_params."""
        ast = parse_lattice("@mixin button($bg, $fg) {}")
        mixin = _find_rule(ast, "mixin_definition")
        params = _find_rule(mixin, "mixin_params")
        param_nodes = _collect_rules(params, "mixin_param")
        assert len(param_nodes) == 2

    def test_mixin_with_default_param(self) -> None:
        """@mixin button($bg, $fg: white) {} parses default values."""
        ast = parse_lattice("@mixin button($bg, $fg: white) {}")
        mixin = _find_rule(ast, "mixin_definition")
        params = _collect_rules(mixin, "mixin_param")
        assert len(params) == 2
        # Defaults intentionally use mixin_value_list so commas remain available
        # as parameter separators.
        second_param = params[1]
        value_list = _collect_rules(second_param, "mixin_value_list")
        assert len(value_list) == 1

    def test_mixin_with_body(self) -> None:
        """@mixin button($bg) { background: $bg; } parses the block."""
        ast = parse_lattice("@mixin button($bg) { background: $bg; }")
        mixin = _find_rule(ast, "mixin_definition")
        block = _find_rule(mixin, "block")
        assert block is not None
        decl = _find_rule(block, "declaration")
        assert decl is not None


# ===========================================================================
# Include Directive Tests
# ===========================================================================


class TestIncludeDirective:
    """Test parsing of @include directives."""

    def test_include_with_args(self) -> None:
        """@include button(red); parses as include_directive.

        button( is a FUNCTION token, so this uses the first alternation:
        "@include" FUNCTION include_args RPAREN SEMICOLON
        """
        ast = parse_lattice("h1 { @include button(red); }")
        include = _find_rule(ast, "include_directive")
        assert include is not None

    def test_include_without_args(self) -> None:
        """@include clearfix; parses as include_directive.

        clearfix is an IDENT token, so this uses the second alternation:
        "@include" IDENT SEMICOLON
        """
        ast = parse_lattice("h1 { @include clearfix; }")
        include = _find_rule(ast, "include_directive")
        assert include is not None

    def test_include_with_multiple_args(self) -> None:
        """@include button(red, white); parses multiple args."""
        ast = parse_lattice("h1 { @include button(red, white); }")
        include = _find_rule(ast, "include_directive")
        include_args = _find_rule(include, "include_args")
        assert include_args is not None

    def test_include_with_block(self) -> None:
        """@include responsive { font-size: 14px; } parses with block.

        Uses the IDENT form (no parens) with a content block.
        """
        ast = parse_lattice("h1 { @include responsive { font-size: 14px; } }")
        include = _find_rule(ast, "include_directive")
        block = _collect_rules(include, "block")
        # The include's own content block
        assert len(block) >= 1


# ===========================================================================
# If Directive Tests
# ===========================================================================


class TestIfDirective:
    """Test parsing of @if / @else if / @else directives."""

    def test_simple_if(self) -> None:
        """@if $theme == dark { ... } parses as if_directive."""
        ast = parse_lattice("h1 { @if $theme == dark { color: white; } }")
        if_dir = _find_rule(ast, "if_directive")
        assert if_dir is not None

    def test_if_else(self) -> None:
        """@if ... {} @else {} parses both branches."""
        source = """
        h1 {
            @if $theme == dark {
                color: white;
            } @else {
                color: black;
            }
        }
        """
        ast = parse_lattice(source)
        if_dir = _find_rule(ast, "if_directive")
        blocks = _collect_rules(if_dir, "block")
        # Two blocks: the @if block and the @else block
        assert len(blocks) == 2

    def test_if_else_if_else(self) -> None:
        """@if ... {} @else if ... {} @else {} parses all branches."""
        source = """
        h1 {
            @if $size == large {
                font-size: 24px;
            } @else if $size == medium {
                font-size: 16px;
            } @else {
                font-size: 12px;
            }
        }
        """
        ast = parse_lattice(source)
        if_dir = _find_rule(ast, "if_directive")
        blocks = _collect_rules(if_dir, "block")
        # Three blocks: @if, @else if, @else
        assert len(blocks) == 3

    def test_if_with_not_equals(self) -> None:
        """@if $x != 0 { ... } uses NOT_EQUALS comparison."""
        ast = parse_lattice("h1 { @if $x != 0 { color: red; } }")
        if_dir = _find_rule(ast, "if_directive")
        comparison = _find_rule(if_dir, "lattice_comparison")
        assert comparison is not None

    def test_if_with_boolean_ops(self) -> None:
        """@if $a and $b { ... } uses boolean AND."""
        ast = parse_lattice("h1 { @if $a and $b { color: red; } }")
        if_dir = _find_rule(ast, "if_directive")
        and_expr = _find_rule(if_dir, "lattice_and_expr")
        assert and_expr is not None


# ===========================================================================
# For Directive Tests
# ===========================================================================


class TestForDirective:
    """Test parsing of @for loop directives."""

    def test_for_through(self) -> None:
        """@for $i from 1 through 12 { ... } parses as for_directive."""
        ast = parse_lattice("h1 { @for $i from 1 through 12 { color: red; } }")
        for_dir = _find_rule(ast, "for_directive")
        assert for_dir is not None

    def test_for_to(self) -> None:
        """@for $i from 1 to 5 { ... } uses 'to' keyword."""
        ast = parse_lattice("h1 { @for $i from 1 to 5 { color: red; } }")
        for_dir = _find_rule(ast, "for_directive")
        assert for_dir is not None

    def test_for_with_expressions(self) -> None:
        """@for $i from $start through $end { ... } uses variable bounds."""
        ast = parse_lattice("h1 { @for $i from $start through $end { color: red; } }")
        for_dir = _find_rule(ast, "for_directive")
        # Should have lattice_expression nodes for bounds
        exprs = _collect_rules(for_dir, "lattice_expression")
        assert len(exprs) == 2


# ===========================================================================
# Each Directive Tests
# ===========================================================================


class TestEachDirective:
    """Test parsing of @each loop directives."""

    def test_simple_each(self) -> None:
        """@each $color in red, green, blue { ... } parses as each_directive."""
        ast = parse_lattice("h1 { @each $color in red, green, blue { color: $color; } }")
        each_dir = _find_rule(ast, "each_directive")
        assert each_dir is not None

    def test_each_list(self) -> None:
        """@each list items are captured in each_list."""
        ast = parse_lattice("h1 { @each $x in a, b, c { color: $x; } }")
        each_list = _find_rule(ast, "each_list")
        assert each_list is not None

    def test_each_multiple_variables(self) -> None:
        """@each $key, $value in ... supports destructuring."""
        ast = parse_lattice("h1 { @each $key, $value in a, b { color: $key; } }")
        each_dir = _find_rule(ast, "each_directive")
        assert each_dir is not None


# ===========================================================================
# Expression Tests
# ===========================================================================


class TestExpressions:
    """Test parsing of Lattice expressions in @if conditions."""

    def test_variable_expression(self) -> None:
        """$x alone is a valid expression."""
        ast = parse_lattice("h1 { @if $x { color: red; } }")
        expr = _find_rule(ast, "lattice_expression")
        primary = _find_rule(expr, "lattice_primary")
        assert primary is not None

    def test_comparison_expression(self) -> None:
        """$x == 10 produces a lattice_comparison."""
        ast = parse_lattice("h1 { @if $x == 10 { color: red; } }")
        comparison = _find_rule(ast, "lattice_comparison")
        assert comparison is not None

    def test_arithmetic_expression(self) -> None:
        """$x + 1 produces a lattice_additive."""
        ast = parse_lattice("h1 { @for $i from $x + 1 through 10 { color: red; } }")
        additive = _find_rule(ast, "lattice_additive")
        assert additive is not None

    def test_multiplicative_expression(self) -> None:
        """$x * 2 produces a lattice_multiplicative."""
        ast = parse_lattice("h1 { @for $i from 1 through $x * 2 { color: red; } }")
        mult = _find_rule(ast, "lattice_multiplicative")
        assert mult is not None

    def test_unary_minus(self) -> None:
        """- $x produces a lattice_unary."""
        ast = parse_lattice("h1 { @if -$x == 0 { color: red; } }")
        unary = _find_rule(ast, "lattice_unary")
        assert unary is not None

    def test_parenthesized_expression(self) -> None:
        """($x + 1) groups an expression."""
        ast = parse_lattice("h1 { @for $i from 1 through ($x + 1) { color: red; } }")
        primary = _find_rule(ast, "lattice_primary")
        assert primary is not None

    def test_or_expression(self) -> None:
        """$a or $b produces a lattice_or_expr."""
        ast = parse_lattice("h1 { @if $a or $b { color: red; } }")
        or_expr = _find_rule(ast, "lattice_or_expr")
        assert or_expr is not None

    def test_boolean_literals(self) -> None:
        """true and false are valid primaries."""
        ast = parse_lattice("h1 { @if true { color: red; } }")
        primary = _find_rule(ast, "lattice_primary")
        assert primary is not None

    def test_function_call_in_expression(self) -> None:
        """length($list) is a valid primary in expressions."""
        ast = parse_lattice("h1 { @if length($list) == 0 { color: red; } }")
        func_call = _find_rule(ast, "function_call")
        assert func_call is not None

    def test_greater_equals(self) -> None:
        """$x >= 10 uses GREATER_EQUALS comparison."""
        ast = parse_lattice("h1 { @if $x >= 10 { color: red; } }")
        comp_op = _find_rule(ast, "comparison_op")
        assert comp_op is not None

    def test_less_equals(self) -> None:
        """$x <= 10 uses LESS_EQUALS comparison."""
        ast = parse_lattice("h1 { @if $x <= 10 { color: red; } }")
        comp_op = _find_rule(ast, "comparison_op")
        assert comp_op is not None


# ===========================================================================
# Function Definition Tests
# ===========================================================================


class TestFunctionDefinition:
    """Test parsing of @function definitions."""

    def test_simple_function(self) -> None:
        """@function spacing($n) { @return $n * 8px; } parses correctly."""
        source = "@function spacing($n) { @return $n * 8px; }"
        ast = parse_lattice(source)
        func = _find_rule(ast, "function_definition")
        assert func is not None

    def test_function_body(self) -> None:
        """Function body contains function_body_item nodes."""
        source = "@function spacing($n) { @return $n * 8px; }"
        ast = parse_lattice(source)
        func_body = _find_rule(ast, "function_body")
        assert func_body is not None

    def test_return_directive(self) -> None:
        """@return inside function body parses as return_directive."""
        source = "@function double($n) { @return $n * 2; }"
        ast = parse_lattice(source)
        ret = _find_rule(ast, "return_directive")
        assert ret is not None

    def test_function_with_variable(self) -> None:
        """Function body can contain variable declarations.

        Note: STAR is not a valid ``value`` token (it conflicts with the
        universal selector in CSS), so we use addition instead.
        """
        source = """
        @function complex($a) {
            $tmp: $a + 2;
            @return $tmp;
        }
        """
        ast = parse_lattice(source)
        func_body = _find_rule(ast, "function_body")
        var_decl = _find_rule(func_body, "variable_declaration")
        assert var_decl is not None

    def test_function_with_control_flow(self) -> None:
        """Function body can contain @if control flow."""
        source = """
        @function abs($n) {
            @if $n >= 0 {
                @return $n;
            } @else {
                @return -$n;
            }
        }
        """
        ast = parse_lattice(source)
        func_body = _find_rule(ast, "function_body")
        if_dir = _find_rule(func_body, "if_directive")
        assert if_dir is not None

    def test_function_with_default_params(self) -> None:
        """@function with default parameter values."""
        source = "@function spacing($n: 1) { @return $n * 8px; }"
        ast = parse_lattice(source)
        param = _find_rule(ast, "mixin_param")
        value_list = _find_rule(param, "mixin_value_list")
        assert value_list is not None


# ===========================================================================
# Use Directive Tests
# ===========================================================================


class TestUseDirective:
    """Test parsing of @use module imports."""

    def test_simple_use(self) -> None:
        """@use "colors"; parses as use_directive."""
        ast = parse_lattice('@use "colors";')
        use = _find_rule(ast, "use_directive")
        assert use is not None

    def test_use_with_alias(self) -> None:
        """@use "utils/mixins" as m; parses with alias."""
        ast = parse_lattice('@use "utils/mixins" as m;')
        use = _find_rule(ast, "use_directive")
        assert use is not None
        # Should contain the "as" literal and IDENT "m"
        tokens = _child_token_values(use)
        assert "m" in tokens


# ===========================================================================
# CSS Passthrough Tests
# ===========================================================================


class TestCSSPassthrough:
    """Test that plain CSS parses correctly through the Lattice parser.

    Since Lattice is a true CSS superset, all valid CSS must parse
    without errors and produce the same AST structure.
    """

    def test_simple_rule(self) -> None:
        """h1 { color: red; } parses as a qualified_rule."""
        ast = parse_lattice("h1 { color: red; }")
        rule = _find_rule(ast, "qualified_rule")
        assert rule is not None

    def test_declaration(self) -> None:
        """Declarations inside rules parse correctly."""
        ast = parse_lattice("h1 { color: red; font-size: 16px; }")
        decls = _collect_rules(ast, "declaration")
        assert len(decls) == 2

    def test_selector_list(self) -> None:
        """Multiple selectors parse as selector_list."""
        ast = parse_lattice("h1, h2, h3 { color: red; }")
        sel_list = _find_rule(ast, "selector_list")
        assert sel_list is not None

    def test_class_selector(self) -> None:
        """.container { } parses with class_selector."""
        ast = parse_lattice(".container { color: red; }")
        class_sel = _find_rule(ast, "class_selector")
        assert class_sel is not None

    def test_id_selector(self) -> None:
        """#main { } parses with id_selector (via HASH token)."""
        ast = parse_lattice("#main { color: red; }")
        id_sel = _find_rule(ast, "id_selector")
        assert id_sel is not None

    def test_pseudo_class(self) -> None:
        """a:hover { } parses with pseudo_class."""
        ast = parse_lattice("a:hover { color: red; }")
        pseudo = _find_rule(ast, "pseudo_class")
        assert pseudo is not None

    def test_pseudo_element(self) -> None:
        """p::before { } parses with pseudo_element."""
        ast = parse_lattice("p::before { content: \"\"; }")
        pseudo_el = _find_rule(ast, "pseudo_element")
        assert pseudo_el is not None

    def test_media_query(self) -> None:
        """@media (max-width: 768px) { ... } parses as at_rule."""
        ast = parse_lattice("@media (max-width: 768px) { h1 { color: red; } }")
        at_rule = _find_rule(ast, "at_rule")
        assert at_rule is not None

    def test_import_rule(self) -> None:
        """@import url("style.css"); parses as at_rule."""
        ast = parse_lattice('@import url("style.css");')
        at_rule = _find_rule(ast, "at_rule")
        assert at_rule is not None

    def test_function_value(self) -> None:
        """rgb(255, 0, 0) in a value parses as function_call."""
        ast = parse_lattice("h1 { color: rgb(255, 0, 0); }")
        func = _find_rule(ast, "function_call")
        assert func is not None

    def test_important(self) -> None:
        """!important parses as priority."""
        ast = parse_lattice("h1 { color: red !important; }")
        priority = _find_rule(ast, "priority")
        assert priority is not None

    def test_custom_property(self) -> None:
        """--main-color: red; uses CUSTOM_PROPERTY."""
        ast = parse_lattice("h1 { --main-color: red; }")
        decl = _find_rule(ast, "declaration")
        prop = _find_rule(decl, "property")
        assert prop is not None

    def test_combinator_child(self) -> None:
        """div > p uses child combinator."""
        ast = parse_lattice("div > p { color: red; }")
        combinator = _find_rule(ast, "combinator")
        assert combinator is not None

    def test_combinator_adjacent(self) -> None:
        """h1 + p uses adjacent sibling combinator."""
        ast = parse_lattice("h1 + p { color: red; }")
        combinator = _find_rule(ast, "combinator")
        assert combinator is not None

    def test_attribute_selector(self) -> None:
        """a[href] parses as attribute_selector."""
        ast = parse_lattice("a[href] { color: blue; }")
        attr_sel = _find_rule(ast, "attribute_selector")
        assert attr_sel is not None

    def test_nested_rule(self) -> None:
        """Nested rules within blocks use CSS nesting (& parent selector)."""
        ast = parse_lattice("div { & > p { color: red; } }")
        rules = _collect_rules(ast, "qualified_rule")
        # Outer div and inner & > p
        assert len(rules) == 2


# ===========================================================================
# Mixed Lattice + CSS Tests
# ===========================================================================


class TestMixedSource:
    """Test realistic Lattice source mixing variables, mixins, and CSS."""

    def test_variable_then_rule(self) -> None:
        """Variable declaration followed by a CSS rule."""
        source = """
        $color: red;
        h1 { color: $color; }
        """
        ast = parse_lattice(source)
        var_decl = _find_rule(ast, "variable_declaration")
        qual_rule = _find_rule(ast, "qualified_rule")
        assert var_decl is not None
        assert qual_rule is not None

    def test_mixin_and_include(self) -> None:
        """Define a mixin at top level, include it inside a rule."""
        source = """
        @mixin bold() {
            font-weight: bold;
        }
        h1 {
            @include bold;
        }
        """
        ast = parse_lattice(source)
        mixin = _find_rule(ast, "mixin_definition")
        include = _find_rule(ast, "include_directive")
        assert mixin is not None
        assert include is not None

    def test_variable_in_mixin(self) -> None:
        """Variables used inside mixin body."""
        source = """
        @mixin theme($bg) {
            background: $bg;
            color: white;
        }
        """
        ast = parse_lattice(source)
        mixin = _find_rule(ast, "mixin_definition")
        block = _find_rule(mixin, "block")
        decls = _collect_rules(block, "declaration")
        assert len(decls) == 2

    def test_control_flow_in_rule(self) -> None:
        """@if inside a qualified rule."""
        source = """
        h1 {
            @if $theme == dark {
                color: white;
            } @else {
                color: black;
            }
        }
        """
        ast = parse_lattice(source)
        if_dir = _find_rule(ast, "if_directive")
        assert if_dir is not None

    def test_for_loop_in_rule(self) -> None:
        """@for inside a qualified rule."""
        source = """
        .grid {
            @for $i from 1 through 12 {
                color: red;
            }
        }
        """
        ast = parse_lattice(source)
        for_dir = _find_rule(ast, "for_directive")
        assert for_dir is not None

    def test_each_loop_in_rule(self) -> None:
        """@each inside a qualified rule."""
        source = """
        .colors {
            @each $c in red, blue, green {
                color: $c;
            }
        }
        """
        ast = parse_lattice(source)
        each_dir = _find_rule(ast, "each_directive")
        assert each_dir is not None

    def test_function_and_usage(self) -> None:
        """Define a function at top level, use it in a value."""
        source = """
        @function double($n) {
            @return $n * 2;
        }
        h1 {
            padding: double(8px);
        }
        """
        ast = parse_lattice(source)
        func_def = _find_rule(ast, "function_definition")
        assert func_def is not None
        # The function call in the value position
        func_calls = _collect_rules(ast, "function_call")
        assert len(func_calls) >= 1

    def test_use_then_variable(self) -> None:
        """@use followed by variable declarations and rules."""
        source = """
        @use "colors";
        $bg: red;
        h1 { background: $bg; }
        """
        ast = parse_lattice(source)
        use = _find_rule(ast, "use_directive")
        var_decl = _find_rule(ast, "variable_declaration")
        assert use is not None
        assert var_decl is not None

    def test_complex_realistic_source(self) -> None:
        """A realistic Lattice file with multiple features."""
        source = """
        @use "colors";

        $base-size: 16px;
        $primary: #4a90d9;

        @mixin button($bg, $fg: white) {
            background: $bg;
            color: $fg;
            padding: 8px 16px;
        }

        @function spacing($n) {
            @return $n * 8px;
        }

        .container {
            max-width: 1200px;
            padding: spacing(2);
        }

        .btn-primary {
            @include button($primary);
        }

        @media (max-width: 768px) {
            .container {
                padding: spacing(1);
            }
        }
        """
        ast = parse_lattice(source)
        assert ast.rule_name == "stylesheet"

        # Verify all top-level constructs were parsed
        use = _find_rule(ast, "use_directive")
        assert use is not None
        var_decls = _collect_rules(ast, "variable_declaration")
        assert len(var_decls) >= 2
        mixin = _find_rule(ast, "mixin_definition")
        assert mixin is not None
        func = _find_rule(ast, "function_definition")
        assert func is not None

    def test_nested_control_flow(self) -> None:
        """@if inside @for — nested control flow."""
        source = """
        .grid {
            @for $i from 1 through 12 {
                @if $i == 6 {
                    color: red;
                }
            }
        }
        """
        ast = parse_lattice(source)
        for_dir = _find_rule(ast, "for_directive")
        if_dir = _find_rule(for_dir, "if_directive")
        assert if_dir is not None


# ===========================================================================
# Comment Tests
# ===========================================================================


class TestComments:
    """Test that comments are properly skipped during parsing."""

    def test_block_comment(self) -> None:
        """/* ... */ comments don't affect parsing."""
        ast = parse_lattice("/* comment */ h1 { color: red; }")
        rule = _find_rule(ast, "qualified_rule")
        assert rule is not None

    def test_line_comment(self) -> None:
        """// comments don't affect parsing (Lattice extension)."""
        ast = parse_lattice("// comment\nh1 { color: red; }")
        rule = _find_rule(ast, "qualified_rule")
        assert rule is not None

    def test_comment_in_block(self) -> None:
        """Comments inside blocks are skipped."""
        source = """
        h1 {
            /* a comment */
            color: red;
            // another comment
            font-size: 16px;
        }
        """
        ast = parse_lattice(source)
        decls = _collect_rules(ast, "declaration")
        assert len(decls) == 2
