"""Tests for the CSS emitter.

The emitter takes a clean CSS AST (no Lattice nodes) and produces CSS text.
Since the Lattice parser can parse plain CSS (Lattice is a CSS superset),
we test by parsing CSS, then emitting it and verifying the output.

Test Strategy
-------------

For each CSS construct, we:

1. Parse the CSS source with the Lattice parser.
2. Emit it with the CSSEmitter.
3. Verify the output contains the expected CSS patterns.

We don't test exact string equality because whitespace normalization
may differ slightly from the input. Instead, we check that key patterns
are present and the output is valid.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from lattice_ast_to_css.emitter import CSSEmitter
from lattice_parser import parse_lattice


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _emit(source: str, minified: bool = False) -> str:
    """Parse CSS source and emit it back."""
    ast = parse_lattice(source)
    emitter = CSSEmitter(minified=minified)
    return emitter.emit(ast)


def _normalize(text: str) -> str:
    """Normalize whitespace for comparison."""
    return " ".join(text.split())


# ---------------------------------------------------------------------------
# Mock types for unit testing individual handlers
# ---------------------------------------------------------------------------


@dataclass
class MockToken:
    type: str
    value: str


@dataclass
class MockNode:
    rule_name: str
    children: list[Any]


# ===========================================================================
# Basic Rule Tests
# ===========================================================================


class TestQualifiedRule:
    """Test emission of qualified rules (selector + block)."""

    def test_simple_rule(self) -> None:
        result = _emit("h1 { color: red; }")
        assert "h1" in result
        assert "color: red;" in result

    def test_two_declarations(self) -> None:
        result = _emit("h1 { color: red; font-size: 16px; }")
        assert "color: red;" in result
        assert "font-size: 16px;" in result

    def test_block_braces(self) -> None:
        result = _emit("h1 { color: red; }")
        assert "{" in result
        assert "}" in result

    def test_indentation(self) -> None:
        """Declarations should be indented inside blocks."""
        result = _emit("h1 { color: red; }")
        lines = result.strip().split("\n")
        # Find the declaration line
        decl_lines = [l for l in lines if "color" in l]
        assert len(decl_lines) > 0
        # It should be indented (start with whitespace)
        assert decl_lines[0].startswith(" ") or decl_lines[0].startswith("\t")


class TestDeclaration:
    """Test emission of CSS declarations."""

    def test_property_value(self) -> None:
        result = _emit("h1 { color: red; }")
        assert "color: red;" in result

    def test_dimension_value(self) -> None:
        result = _emit("h1 { font-size: 16px; }")
        assert "font-size: 16px;" in result

    def test_percentage_value(self) -> None:
        result = _emit("h1 { width: 50%; }")
        assert "width: 50%;" in result

    def test_number_value(self) -> None:
        result = _emit("h1 { opacity: 0; }")
        assert "opacity: 0;" in result

    def test_hash_value(self) -> None:
        result = _emit("h1 { color: #fff; }")
        assert "color: #fff;" in result

    def test_multi_value(self) -> None:
        result = _emit("h1 { margin: 10px 20px; }")
        assert "margin:" in result
        assert "10px" in result
        assert "20px" in result

    def test_important(self) -> None:
        result = _emit("h1 { color: red !important; }")
        assert "!important" in result

    def test_custom_property(self) -> None:
        result = _emit("h1 { --main-color: red; }")
        assert "--main-color" in result
        assert "red" in result


class TestFunctionValues:
    """Test emission of CSS function calls in values."""

    def test_rgb(self) -> None:
        result = _emit("h1 { color: rgb(255, 0, 0); }")
        assert "rgb(" in result
        assert ")" in result

    def test_url(self) -> None:
        result = _emit("h1 { background: url(image.png); }")
        assert "url(image.png)" in result

    def test_calc(self) -> None:
        result = _emit("h1 { width: calc(100% - 20px); }")
        assert "calc(" in result


# ===========================================================================
# Selector Tests
# ===========================================================================


class TestSelectors:
    """Test emission of various CSS selectors."""

    def test_type_selector(self) -> None:
        result = _emit("h1 { color: red; }")
        assert "h1" in result

    def test_class_selector(self) -> None:
        result = _emit(".container { color: red; }")
        assert ".container" in result

    def test_id_selector(self) -> None:
        result = _emit("#main { color: red; }")
        assert "#main" in result

    def test_selector_list(self) -> None:
        result = _emit("h1, h2, h3 { color: red; }")
        assert "h1" in result
        assert "h2" in result
        assert "h3" in result

    def test_child_combinator(self) -> None:
        result = _emit("div > p { color: red; }")
        assert ">" in result

    def test_adjacent_combinator(self) -> None:
        result = _emit("h1 + p { color: red; }")
        assert "+" in result

    def test_pseudo_class(self) -> None:
        result = _emit("a:hover { color: red; }")
        assert ":hover" in result

    def test_pseudo_element(self) -> None:
        result = _emit('p::before { content: ""; }')
        assert "::before" in result

    def test_attribute_selector(self) -> None:
        result = _emit("a[href] { color: blue; }")
        assert "[href]" in result

    def test_universal_selector(self) -> None:
        result = _emit("* { margin: 0; }")
        assert "*" in result

    def test_ampersand_parent(self) -> None:
        """CSS nesting parent selector."""
        result = _emit("div { & > p { color: red; } }")
        assert "&" in result


# ===========================================================================
# At-Rule Tests
# ===========================================================================


class TestAtRules:
    """Test emission of CSS at-rules."""

    def test_import(self) -> None:
        result = _emit('@import url("style.css");')
        assert "@import" in result
        assert ";" in result

    def test_media(self) -> None:
        result = _emit("@media (max-width: 768px) { h1 { color: red; } }")
        assert "@media" in result
        assert "max-width" in result

    def test_charset(self) -> None:
        result = _emit('@charset "UTF-8";')
        assert "@charset" in result

    def test_keyframes(self) -> None:
        source = "@keyframes fade { h1 { opacity: 0; } }"
        result = _emit(source)
        assert "@keyframes" in result
        assert "fade" in result


# ===========================================================================
# Multiple Rules
# ===========================================================================


class TestMultipleRules:
    """Test emission of stylesheets with multiple rules."""

    def test_two_rules(self) -> None:
        result = _emit("h1 { color: red; }\nh2 { color: blue; }")
        assert "h1" in result
        assert "h2" in result
        assert "red" in result
        assert "blue" in result

    def test_rules_separated(self) -> None:
        """In pretty mode, rules should be separated by blank lines."""
        result = _emit("h1 { color: red; }\nh2 { color: blue; }")
        assert "\n\n" in result

    def test_empty_stylesheet(self) -> None:
        result = _emit("")
        assert result == ""


# ===========================================================================
# Minified Mode Tests
# ===========================================================================


class TestMinified:
    """Test minified CSS output."""

    def test_no_extra_whitespace(self) -> None:
        result = _emit("h1 { color: red; }", minified=True)
        # Minified should have no unnecessary whitespace
        assert "\n" not in result.strip()

    def test_no_blank_lines(self) -> None:
        result = _emit("h1 { color: red; }\nh2 { color: blue; }", minified=True)
        assert "\n\n" not in result

    def test_compact_declarations(self) -> None:
        result = _emit("h1 { color: red; font-size: 16px; }", minified=True)
        assert "color:" in result
        assert "font-size:" in result


# ===========================================================================
# Round-Trip Tests
# ===========================================================================


class TestRoundTrip:
    """Test that parse → emit produces valid CSS.

    We can't test exact equality because whitespace normalization may differ.
    Instead, we verify that the emitted CSS, when re-parsed, produces an AST
    that emits the same CSS again (idempotency of emit).
    """

    def test_simple_rule_idempotent(self) -> None:
        first = _emit("h1 { color: red; }")
        second = _emit(first)
        assert _normalize(first) == _normalize(second)

    def test_multi_rule_idempotent(self) -> None:
        first = _emit("h1 { color: red; }\nh2 { font-size: 16px; }")
        second = _emit(first)
        assert _normalize(first) == _normalize(second)

    def test_media_query_idempotent(self) -> None:
        source = "@media (max-width: 768px) { h1 { color: red; } }"
        first = _emit(source)
        second = _emit(first)
        assert _normalize(first) == _normalize(second)

    def test_complex_selectors_idempotent(self) -> None:
        source = "div > p + span { color: red; }"
        first = _emit(source)
        second = _emit(first)
        assert _normalize(first) == _normalize(second)

    def test_multiple_declarations_idempotent(self) -> None:
        source = "h1 { color: red; font-size: 16px; margin: 0; padding: 10px 20px; }"
        first = _emit(source)
        second = _emit(first)
        assert _normalize(first) == _normalize(second)


# ===========================================================================
# Direct Node Tests (unit testing handlers)
# ===========================================================================


class TestDirectEmission:
    """Test emitter methods directly with mock nodes."""

    def test_emit_empty_node(self) -> None:
        emitter = CSSEmitter()
        node = MockNode("stylesheet", [])
        assert emitter.emit(node) == ""

    def test_emit_token(self) -> None:
        emitter = CSSEmitter()
        result = emitter._emit_node(MockToken("IDENT", "red"))
        assert result == "red"

    def test_emit_property(self) -> None:
        emitter = CSSEmitter()
        node = MockNode("property", [MockToken("IDENT", "color")])
        result = emitter._emit_property(node, 0)
        assert result == "color"

    def test_emit_custom_property(self) -> None:
        emitter = CSSEmitter()
        node = MockNode("property", [MockToken("CUSTOM_PROPERTY", "--main-color")])
        result = emitter._emit_property(node, 0)
        assert result == "--main-color"
