"""Tests for the CSS parser thin wrapper.

These tests verify that the grammar-driven parser, configured with
``css.grammar``, correctly parses CSS text into ASTs. CSS is the most
complex grammar in the collection — these tests exercise selectors,
at-rules, values, nesting, and error handling.
"""

from __future__ import annotations

import pytest

from css_parser import create_css_parser, parse_css
from lang_parser import ASTNode, GrammarParser, GrammarParseError
from lexer import Token


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def get_type_name(token: Token) -> str:
    """Extract the type name from a token (handles both enum and string)."""
    return token.type if isinstance(token.type, str) else token.type.name


def find_nodes(node: ASTNode, rule_name: str) -> list[ASTNode]:
    """Recursively find all nodes with a given rule_name."""
    results: list[ASTNode] = []
    if node.rule_name == rule_name:
        results.append(node)
    for child in node.children:
        if isinstance(child, ASTNode):
            results.extend(find_nodes(child, rule_name))
    return results


def find_tokens(node: ASTNode, token_type: str) -> list[Token]:
    """Recursively find all tokens of a given type in the AST."""
    results: list[Token] = []
    for child in node.children:
        if isinstance(child, Token) and get_type_name(child) == token_type:
            results.append(child)
        elif isinstance(child, ASTNode):
            results.extend(find_tokens(child, token_type))
    return results


def child_tokens(node: ASTNode) -> list[Token]:
    """Extract all Token children from a node (not ASTNode children)."""
    return [c for c in node.children if isinstance(c, Token)]


def child_nodes(node: ASTNode) -> list[ASTNode]:
    """Extract all ASTNode children from a node (not Token children)."""
    return [c for c in node.children if isinstance(c, ASTNode)]


# ---------------------------------------------------------------------------
# Factory function tests
# ---------------------------------------------------------------------------


class TestFactory:
    """Tests for the create_css_parser factory function."""

    def test_returns_grammar_parser(self) -> None:
        """create_css_parser should return a GrammarParser instance."""
        parser = create_css_parser("h1 { color: red; }")
        assert isinstance(parser, GrammarParser)

    def test_factory_produces_ast(self) -> None:
        """The factory-created parser should produce a valid AST."""
        parser = create_css_parser("h1 { color: red; }")
        ast = parser.parse()
        assert isinstance(ast, ASTNode)
        assert ast.rule_name == "stylesheet"


# ---------------------------------------------------------------------------
# Empty and minimal stylesheets
# ---------------------------------------------------------------------------


class TestMinimal:
    """Tests for empty and minimal CSS inputs."""

    def test_empty_stylesheet(self) -> None:
        """An empty stylesheet produces a stylesheet node with no children."""
        ast = parse_css("")
        assert ast.rule_name == "stylesheet"
        assert len(ast.children) == 0

    def test_single_rule(self) -> None:
        """A single rule with one declaration."""
        ast = parse_css("h1 { color: red; }")
        assert ast.rule_name == "stylesheet"
        rule_nodes = find_nodes(ast, "rule")
        assert len(rule_nodes) == 1

    def test_multiple_rules(self) -> None:
        """Multiple rules in a stylesheet."""
        ast = parse_css("h1 { color: red; } p { margin: 0; }")
        rule_nodes = find_nodes(ast, "rule")
        assert len(rule_nodes) == 2

    def test_whitespace_only(self) -> None:
        """A stylesheet with only whitespace is empty."""
        ast = parse_css("   \n\t  \n  ")
        assert ast.rule_name == "stylesheet"
        assert len(ast.children) == 0


# ---------------------------------------------------------------------------
# Simple selector tests
# ---------------------------------------------------------------------------


class TestSimpleSelectors:
    """Tests for basic CSS selectors."""

    def test_type_selector(self) -> None:
        """A type selector (element name) like ``h1``."""
        ast = parse_css("h1 { color: red; }")
        selectors = find_nodes(ast, "simple_selector")
        assert len(selectors) == 1
        tokens = child_tokens(selectors[0])
        assert tokens[0].value == "h1"

    def test_universal_selector(self) -> None:
        """The universal selector ``*``."""
        ast = parse_css("* { margin: 0; }")
        selectors = find_nodes(ast, "simple_selector")
        assert len(selectors) == 1
        tokens = child_tokens(selectors[0])
        assert tokens[0].value == "*"

    def test_class_selector(self) -> None:
        """A class selector ``.container``."""
        ast = parse_css(".container { margin: 0; }")
        class_selectors = find_nodes(ast, "class_selector")
        assert len(class_selectors) == 1
        tokens = child_tokens(class_selectors[0])
        # DOT + IDENT
        assert len(tokens) == 2
        assert tokens[1].value == "container"

    def test_id_selector(self) -> None:
        """An ID selector ``#header``."""
        ast = parse_css("#header { padding: 0; }")
        id_selectors = find_nodes(ast, "id_selector")
        assert len(id_selectors) == 1
        tokens = child_tokens(id_selectors[0])
        assert tokens[0].value == "#header"

    def test_compound_selector(self) -> None:
        """A compound selector ``div.class#id``."""
        ast = parse_css("div.active#main { display: block; }")
        simple = find_nodes(ast, "simple_selector")
        assert len(simple) == 1
        class_sel = find_nodes(ast, "class_selector")
        assert len(class_sel) == 1
        id_sel = find_nodes(ast, "id_selector")
        assert len(id_sel) == 1

    def test_multiple_classes(self) -> None:
        """Multiple class selectors ``.a.b.c``."""
        ast = parse_css(".a.b.c { color: red; }")
        class_selectors = find_nodes(ast, "class_selector")
        assert len(class_selectors) == 3


# ---------------------------------------------------------------------------
# Combinator tests
# ---------------------------------------------------------------------------


class TestCombinators:
    """Tests for CSS combinator selectors."""

    def test_descendant_combinator(self) -> None:
        """Descendant combinator (whitespace): ``div p``."""
        ast = parse_css("div p { color: red; }")
        compound = find_nodes(ast, "compound_selector")
        # Two compound selectors: div and p
        assert len(compound) == 2

    def test_child_combinator(self) -> None:
        """Child combinator ``>``: ``div > p``."""
        ast = parse_css("div > p { color: red; }")
        combinators = find_nodes(ast, "combinator")
        assert len(combinators) == 1
        tokens = child_tokens(combinators[0])
        assert tokens[0].value == ">"

    def test_adjacent_sibling(self) -> None:
        """Adjacent sibling combinator ``+``: ``h1 + p``."""
        ast = parse_css("h1 + p { color: red; }")
        combinators = find_nodes(ast, "combinator")
        assert len(combinators) == 1
        tokens = child_tokens(combinators[0])
        assert tokens[0].value == "+"

    def test_general_sibling(self) -> None:
        """General sibling combinator ``~``: ``h1 ~ p``."""
        ast = parse_css("h1 ~ p { color: red; }")
        combinators = find_nodes(ast, "combinator")
        assert len(combinators) == 1
        tokens = child_tokens(combinators[0])
        assert tokens[0].value == "~"

    def test_chained_combinators(self) -> None:
        """Chained combinators: ``div > p + span``."""
        ast = parse_css("div > p + span { color: red; }")
        combinators = find_nodes(ast, "combinator")
        assert len(combinators) == 2


# ---------------------------------------------------------------------------
# Selector list tests
# ---------------------------------------------------------------------------


class TestSelectorList:
    """Tests for comma-separated selector lists."""

    def test_two_selectors(self) -> None:
        """Two selectors: ``h1, h2``."""
        ast = parse_css("h1, h2 { color: red; }")
        complex_selectors = find_nodes(ast, "complex_selector")
        assert len(complex_selectors) == 2

    def test_three_selectors(self) -> None:
        """Three selectors: ``h1, h2, h3``."""
        ast = parse_css("h1, h2, h3 { font-weight: bold; }")
        complex_selectors = find_nodes(ast, "complex_selector")
        assert len(complex_selectors) == 3

    def test_mixed_selector_types(self) -> None:
        """Mixed selector types in a list: ``h1, .class, #id``."""
        ast = parse_css("h1, .active, #main { display: block; }")
        complex_selectors = find_nodes(ast, "complex_selector")
        assert len(complex_selectors) == 3


# ---------------------------------------------------------------------------
# Attribute selector tests
# ---------------------------------------------------------------------------


class TestAttributeSelectors:
    """Tests for CSS attribute selectors."""

    def test_has_attribute(self) -> None:
        """Presence attribute selector: ``[disabled]``."""
        ast = parse_css("[disabled] { opacity: 0.5; }")
        attr = find_nodes(ast, "attribute_selector")
        assert len(attr) == 1

    def test_exact_match(self) -> None:
        """Exact match: ``[type=\"text\"]``."""
        ast = parse_css('[type="text"] { border: 1px; }')
        attr = find_nodes(ast, "attribute_selector")
        assert len(attr) == 1
        matchers = find_nodes(ast, "attr_matcher")
        assert len(matchers) == 1

    def test_substring_match(self) -> None:
        """Substring match: ``[class*=\"warning\"]``."""
        ast = parse_css('[class*="warning"] { color: red; }')
        attr = find_nodes(ast, "attribute_selector")
        assert len(attr) == 1


# ---------------------------------------------------------------------------
# Pseudo-class and pseudo-element tests
# ---------------------------------------------------------------------------


class TestPseudoSelectors:
    """Tests for pseudo-class and pseudo-element selectors."""

    def test_simple_pseudo_class(self) -> None:
        """Simple pseudo-class: ``a:hover``."""
        ast = parse_css("a:hover { color: blue; }")
        pseudo = find_nodes(ast, "pseudo_class")
        assert len(pseudo) == 1

    def test_functional_pseudo_class(self) -> None:
        """Functional pseudo-class: ``:nth-child(2)``."""
        ast = parse_css("li:nth-child(2) { color: red; }")
        pseudo = find_nodes(ast, "pseudo_class")
        assert len(pseudo) == 1

    def test_pseudo_element(self) -> None:
        """Pseudo-element: ``p::before``."""
        ast = parse_css('p::before { content: "hello"; }')
        pseudo = find_nodes(ast, "pseudo_element")
        assert len(pseudo) == 1

    def test_pseudo_element_after(self) -> None:
        """Pseudo-element: ``p::after``."""
        ast = parse_css('p::after { content: "!"; }')
        pseudo = find_nodes(ast, "pseudo_element")
        assert len(pseudo) == 1


# ---------------------------------------------------------------------------
# Declaration tests
# ---------------------------------------------------------------------------


class TestDeclarations:
    """Tests for CSS declarations (property: value pairs)."""

    def test_single_declaration(self) -> None:
        """A single declaration: ``color: red``."""
        ast = parse_css("h1 { color: red; }")
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 1
        # Property should be IDENT "color"
        props = find_nodes(decls[0], "property")
        assert len(props) == 1
        tokens = child_tokens(props[0])
        assert tokens[0].value == "color"

    def test_multiple_declarations(self) -> None:
        """Multiple declarations in one rule."""
        ast = parse_css("h1 { color: red; font-size: 16px; margin: 0; }")
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 3

    def test_dimension_value(self) -> None:
        """A dimension value like ``16px``."""
        ast = parse_css("h1 { font-size: 16px; }")
        dim_tokens = find_tokens(ast, "DIMENSION")
        assert len(dim_tokens) == 1
        assert dim_tokens[0].value == "16px"

    def test_percentage_value(self) -> None:
        """A percentage value like ``50%``."""
        ast = parse_css("div { width: 50%; }")
        pct_tokens = find_tokens(ast, "PERCENTAGE")
        assert len(pct_tokens) == 1
        assert pct_tokens[0].value == "50%"

    def test_number_value(self) -> None:
        """A bare number value like ``0``."""
        ast = parse_css("div { margin: 0; }")
        num_tokens = find_tokens(ast, "NUMBER")
        assert len(num_tokens) == 1
        assert num_tokens[0].value == "0"

    def test_hash_color(self) -> None:
        """A hash color value like ``#fff``."""
        ast = parse_css("h1 { color: #fff; }")
        hash_tokens = find_tokens(ast, "HASH")
        assert len(hash_tokens) == 1
        assert hash_tokens[0].value == "#fff"

    def test_string_value(self) -> None:
        """A string value in quotes."""
        ast = parse_css('h1 { content: "hello"; }')
        str_tokens = find_tokens(ast, "STRING")
        assert len(str_tokens) == 1
        assert str_tokens[0].value == "hello"

    def test_important_priority(self) -> None:
        """The ``!important`` priority annotation."""
        ast = parse_css("h1 { color: red !important; }")
        priority = find_nodes(ast, "priority")
        assert len(priority) == 1
        # Should contain BANG and literal "important"
        tokens = child_tokens(priority[0])
        assert any(t.value == "!" for t in tokens)
        assert any(t.value == "important" for t in tokens)

    def test_custom_property_declaration(self) -> None:
        """A custom property declaration: ``--main-color: #333``."""
        ast = parse_css(":root { --main-color: #333; }")
        props = find_nodes(ast, "property")
        assert len(props) == 1
        tokens = child_tokens(props[0])
        assert tokens[0].value == "--main-color"
        assert get_type_name(tokens[0]) == "CUSTOM_PROPERTY"

    def test_multiple_values(self) -> None:
        """A shorthand property with multiple values: ``margin: 10px 20px``."""
        ast = parse_css("div { margin: 10px 20px; }")
        value_list = find_nodes(ast, "value_list")
        assert len(value_list) == 1
        values = find_nodes(value_list[0], "value")
        assert len(values) == 2


# ---------------------------------------------------------------------------
# Function call tests
# ---------------------------------------------------------------------------


class TestFunctionCalls:
    """Tests for CSS function calls in values."""

    def test_rgb_function(self) -> None:
        """The ``rgb()`` function: ``rgb(255, 0, 0)``."""
        ast = parse_css("h1 { color: rgb(255, 0, 0); }")
        func_calls = find_nodes(ast, "function_call")
        assert len(func_calls) == 1

    def test_calc_function(self) -> None:
        """The ``calc()`` function: ``calc(100% - 20px)``."""
        ast = parse_css("div { width: calc(100% - 20px); }")
        func_calls = find_nodes(ast, "function_call")
        assert len(func_calls) == 1

    def test_var_function(self) -> None:
        """The ``var()`` function with fallback: ``var(--color, blue)``."""
        ast = parse_css("h1 { color: var(--main-color, blue); }")
        func_calls = find_nodes(ast, "function_call")
        assert len(func_calls) == 1

    def test_url_token(self) -> None:
        """The unquoted ``url()`` function: ``url(image.png)``."""
        ast = parse_css("div { background: url(image.png); }")
        url_tokens = find_tokens(ast, "URL_TOKEN")
        assert len(url_tokens) == 1
        assert url_tokens[0].value == "url(image.png)"

    def test_nested_functions(self) -> None:
        """Nested function calls: ``calc(100% - var(--gap))``."""
        ast = parse_css("div { width: calc(100% - var(--gap)); }")
        func_calls = find_nodes(ast, "function_call")
        # calc() is a function_call, but var(--gap) inside is a nested
        # function_arg -> FUNCTION function_args RPAREN
        assert len(func_calls) >= 1


# ---------------------------------------------------------------------------
# At-rule tests
# ---------------------------------------------------------------------------


class TestAtRules:
    """Tests for CSS at-rules."""

    def test_import(self) -> None:
        """``@import`` with string prelude and semicolon."""
        ast = parse_css('@import "style.css";')
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1
        at_kw = find_tokens(ast, "AT_KEYWORD")
        assert at_kw[0].value == "@import"

    def test_charset(self) -> None:
        """``@charset`` with string prelude and semicolon."""
        ast = parse_css('@charset "UTF-8";')
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1

    def test_media_with_block(self) -> None:
        """``@media`` with prelude and block containing rules."""
        ast = parse_css("@media screen { h1 { color: red; } }")
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1
        # The block should contain a qualified rule
        blocks = find_nodes(ast, "block")
        assert len(blocks) >= 1

    def test_media_complex_query(self) -> None:
        """``@media`` with a complex media query prelude."""
        ast = parse_css(
            "@media screen and (min-width: 768px) { .container { width: 750px; } }"
        )
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1

    def test_keyframes(self) -> None:
        """``@keyframes`` with a name and empty block."""
        ast = parse_css("@keyframes fadeIn { }")
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1
        at_kw = find_tokens(ast, "AT_KEYWORD")
        assert at_kw[0].value == "@keyframes"

    def test_font_face(self) -> None:
        """``@font-face`` with declarations inside."""
        ast = parse_css("@font-face { font-family: MyFont; }")
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 1

    def test_multiple_at_rules(self) -> None:
        """Multiple at-rules in a stylesheet."""
        ast = parse_css('@charset "UTF-8"; @import "base.css";')
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 2


# ---------------------------------------------------------------------------
# CSS Nesting tests
# ---------------------------------------------------------------------------


class TestNesting:
    """Tests for CSS Nesting with the ``&`` selector."""

    def test_nested_rule(self) -> None:
        """A nested rule using ``&``."""
        ast = parse_css(".parent { color: red; & .child { color: blue; } }")
        # Should have the parent qualified_rule and a nested one
        qualified = find_nodes(ast, "qualified_rule")
        assert len(qualified) == 2

    def test_nested_with_combinator(self) -> None:
        """A nested rule with a child combinator: ``& > .child``."""
        ast = parse_css(".parent { & > .child { color: blue; } }")
        qualified = find_nodes(ast, "qualified_rule")
        assert len(qualified) == 2
        combinators = find_nodes(ast, "combinator")
        assert len(combinators) == 1

    def test_nested_at_rule(self) -> None:
        """A nested at-rule inside a qualified rule."""
        ast = parse_css(
            ".container { color: red; @media print { color: black; } }"
        )
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1


# ---------------------------------------------------------------------------
# Complex real-world CSS tests
# ---------------------------------------------------------------------------


class TestRealWorldCSS:
    """Tests for realistic CSS documents."""

    def test_multi_rule_stylesheet(self) -> None:
        """A stylesheet with multiple rule types."""
        css = """
            @charset "UTF-8";
            @import "base.css";

            * { margin: 0; padding: 0; }

            h1 { font-size: 24px; color: #333; }

            .container { width: 960px; margin: 0; }

            @media screen and (max-width: 768px) {
                .container { width: 100%; }
            }
        """
        ast = parse_css(css)
        assert ast.rule_name == "stylesheet"
        rule_nodes = find_nodes(ast, "rule")
        # @charset, @import, *, h1, .container, @media (which contains .container)
        assert len(rule_nodes) >= 5

    def test_complex_selectors(self) -> None:
        """A rule with complex selectors."""
        css = "div.container > ul.nav li.active a:hover { color: #00f; }"
        ast = parse_css(css)
        compound = find_nodes(ast, "compound_selector")
        assert len(compound) >= 4  # div.container, ul.nav, li.active, a:hover

    def test_font_shorthand(self) -> None:
        """A font shorthand with slash separator."""
        ast = parse_css("p { font: 16px / 1.5 Arial; }")
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 1
        # Should have dimension, slash, number, ident
        value_list = find_nodes(decls[0], "value_list")
        assert len(value_list) == 1

    def test_vendor_prefix(self) -> None:
        """Vendor-prefixed properties and values."""
        ast = parse_css("div { -webkit-transform: rotate(45deg); }")
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 1
        props = find_nodes(decls[0], "property")
        tokens = child_tokens(props[0])
        assert tokens[0].value == "-webkit-transform"

    def test_multiline_css(self) -> None:
        """A multiline stylesheet with comments."""
        css = """
            /* Navigation styles */
            nav {
                display: flex;
                justify-content: space-between;
            }

            /* Links */
            nav a {
                text-decoration: none;
                color: #333;
            }
        """
        ast = parse_css(css)
        rule_nodes = find_nodes(ast, "rule")
        assert len(rule_nodes) == 2


# ---------------------------------------------------------------------------
# Error case tests
# ---------------------------------------------------------------------------


class TestErrors:
    """Tests for CSS parse errors."""

    def test_missing_closing_brace(self) -> None:
        """Missing closing brace should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_css("h1 { color: red;")

    def test_missing_semicolon(self) -> None:
        """Missing semicolon after declaration should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_css("h1 { color: red }")

    def test_missing_colon(self) -> None:
        """Missing colon in declaration should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_css("h1 { color red; }")

    def test_missing_value(self) -> None:
        """Missing value in declaration should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_css("h1 { color: ; }")

    def test_invalid_at_rule(self) -> None:
        """An at-rule missing both semicolon and block should raise an error."""
        with pytest.raises(GrammarParseError):
            parse_css("@import")


# ---------------------------------------------------------------------------
# Edge case tests
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Tests for CSS edge cases that stress the parser."""

    def test_empty_block(self) -> None:
        """A rule with an empty block."""
        ast = parse_css("h1 { }")
        blocks = find_nodes(ast, "block")
        assert len(blocks) == 1
        contents = find_nodes(blocks[0], "block_contents")
        assert len(contents) == 1
        # Block contents should have no block_items
        items = find_nodes(contents[0], "block_item")
        assert len(items) == 0

    def test_multiple_values_in_shorthand(self) -> None:
        """A shorthand with four values: ``margin: 10px 20px 30px 40px``."""
        ast = parse_css("div { margin: 10px 20px 30px 40px; }")
        value_list = find_nodes(ast, "value_list")
        assert len(value_list) == 1
        values = find_nodes(value_list[0], "value")
        assert len(values) == 4

    def test_comma_separated_values(self) -> None:
        """Comma-separated values like ``font-family: Arial, Helvetica``."""
        ast = parse_css("p { font-family: Arial, Helvetica; }")
        decls = find_nodes(ast, "declaration")
        assert len(decls) == 1

    def test_deeply_nested_media(self) -> None:
        """A media query with multiple rules inside."""
        css = """
            @media print {
                h1 { color: black; }
                p { font-size: 12pt; }
                .no-print { display: none; }
            }
        """
        ast = parse_css(css)
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1

    def test_at_rule_empty_block(self) -> None:
        """An at-rule with an empty block."""
        ast = parse_css("@media screen { }")
        at_rules = find_nodes(ast, "at_rule")
        assert len(at_rules) == 1

    def test_negative_value(self) -> None:
        """A negative dimension value: ``margin: -10px``."""
        ast = parse_css("div { margin: -10px; }")
        dim_tokens = find_tokens(ast, "DIMENSION")
        assert len(dim_tokens) == 1
        assert dim_tokens[0].value == "-10px"

    def test_decimal_value(self) -> None:
        """A decimal dimension value: ``opacity: 0.5``."""
        ast = parse_css("div { line-height: 1.5em; }")
        dim_tokens = find_tokens(ast, "DIMENSION")
        assert len(dim_tokens) == 1
        assert dim_tokens[0].value == "1.5em"
