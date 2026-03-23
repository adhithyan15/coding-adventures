"""Tests for the Lattice AST transformer.

The transformer is the core of the Lattice-to-CSS compiler. It takes a
Lattice AST and produces a clean CSS AST by expanding all Lattice constructs.

Test Strategy
-------------

We test the full pipeline: parse Lattice source → transform → emit CSS.
This exercises the transformer in its real context, verifying that the
output is correct CSS.

For error cases, we verify that the correct exception type is raised.
"""

from __future__ import annotations

import pytest

from lattice_ast_to_css.emitter import CSSEmitter
from lattice_ast_to_css.errors import (
    CircularReferenceError,
    MissingReturnError,
    UndefinedMixinError,
    UndefinedVariableError,
    WrongArityError,
)
from lattice_ast_to_css.transformer import LatticeTransformer
from lattice_parser import parse_lattice


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _compile(source: str) -> str:
    """Full pipeline: parse → transform → emit."""
    ast = parse_lattice(source)
    transformer = LatticeTransformer()
    css_ast = transformer.transform(ast)
    emitter = CSSEmitter()
    return emitter.emit(css_ast)


def _normalize(text: str) -> str:
    """Normalize whitespace for comparison."""
    return " ".join(text.split())


# ===========================================================================
# Variable Tests
# ===========================================================================


class TestVariables:
    """Test variable declaration and substitution."""

    def test_simple_variable(self) -> None:
        """$color: red; → substitute in value positions."""
        result = _compile("""
            $color: red;
            h1 { color: $color; }
        """)
        assert "color: red;" in result
        # Variable declaration should not appear in output
        assert "$color" not in result

    def test_hash_variable(self) -> None:
        """$color: #4a90d9; → substitutes hash value."""
        result = _compile("""
            $color: #4a90d9;
            h1 { color: $color; }
        """)
        assert "#4a90d9" in result

    def test_dimension_variable(self) -> None:
        """$size: 16px; → substitutes dimension value."""
        result = _compile("""
            $size: 16px;
            h1 { font-size: $size; }
        """)
        assert "16px" in result

    def test_multiple_variables(self) -> None:
        """Multiple variables substitute correctly."""
        result = _compile("""
            $color: red;
            $size: 16px;
            h1 { color: $color; font-size: $size; }
        """)
        assert "color: red;" in result
        assert "font-size: 16px;" in result

    def test_variable_removes_declaration(self) -> None:
        """Variable declarations don't appear in CSS output."""
        result = _compile("$x: red; h1 { color: $x; }")
        # No $x in output
        assert "$" not in result

    def test_variable_in_block(self) -> None:
        """Variables declared inside blocks are local."""
        result = _compile("""
            $color: red;
            h1 {
                $color: blue;
                color: $color;
            }
        """)
        assert "color: blue;" in result


class TestVariableErrors:
    """Test error handling for variables."""

    def test_undefined_variable(self) -> None:
        """Referencing an undefined variable raises UndefinedVariableError."""
        with pytest.raises(UndefinedVariableError):
            _compile("h1 { color: $nonexistent; }")


# ===========================================================================
# Mixin Tests
# ===========================================================================


class TestMixins:
    """Test mixin definition and inclusion."""

    def test_simple_mixin(self) -> None:
        """@mixin with no params, @include expands body."""
        result = _compile("""
            @mixin bold() {
                font-weight: bold;
            }
            h1 { @include bold; }
        """)
        assert "font-weight: bold;" in result
        assert "@mixin" not in result
        assert "@include" not in result

    def test_mixin_with_param(self) -> None:
        """@mixin with params, @include passes args."""
        result = _compile("""
            @mixin text-color($c) {
                color: $c;
            }
            h1 { @include text-color(red); }
        """)
        assert "color: red;" in result

    def test_mixin_with_default(self) -> None:
        """@mixin with default param, @include uses the IDENT form.

        Note: @include name() with empty parens doesn't match include_directive
        because include_args requires at least one value_list. Use the IDENT form
        (no parens) instead — the transformer treats missing args as "use defaults".
        """
        result = _compile("""
            @mixin text-color($c: blue) {
                color: $c;
            }
            h1 { @include text-color; }
        """)
        assert "color: blue;" in result

    def test_mixin_with_multiple_params(self) -> None:
        """@mixin with multiple params."""
        result = _compile("""
            @mixin box($bg, $fg) {
                background: $bg;
                color: $fg;
            }
            h1 { @include box(red, white); }
        """)
        assert "background: red;" in result
        assert "color: white;" in result

    def test_mixin_multiple_includes(self) -> None:
        """Same mixin included multiple times."""
        result = _compile("""
            @mixin bold() {
                font-weight: bold;
            }
            h1 { @include bold; }
            h2 { @include bold; }
        """)
        assert result.count("font-weight: bold;") == 2


class TestMixinErrors:
    """Test error handling for mixins."""

    def test_undefined_mixin(self) -> None:
        with pytest.raises(UndefinedMixinError):
            _compile("h1 { @include nonexistent; }")

    def test_circular_mixin(self) -> None:
        """Circular mixin reference raises CircularReferenceError."""
        with pytest.raises(CircularReferenceError):
            _compile("""
                @mixin a() { @include b; }
                @mixin b() { @include a; }
                h1 { @include a; }
            """)

    def test_wrong_arity(self) -> None:
        with pytest.raises(WrongArityError):
            _compile("""
                @mixin box($a, $b) { color: $a; }
                h1 { @include box(red, blue, green); }
            """)


# ===========================================================================
# Control Flow Tests
# ===========================================================================


class TestIfDirective:
    """Test @if / @else if / @else."""

    def test_if_true(self) -> None:
        """@if with true condition includes the block."""
        result = _compile("""
            $theme: dark;
            h1 {
                @if $theme == dark {
                    color: white;
                }
            }
        """)
        assert "color: white;" in result

    def test_if_false(self) -> None:
        """@if with false condition skips the block."""
        result = _compile("""
            $theme: light;
            h1 {
                @if $theme == dark {
                    color: white;
                }
            }
        """)
        assert "white" not in result

    def test_if_else(self) -> None:
        """@if false falls through to @else."""
        result = _compile("""
            $theme: light;
            h1 {
                @if $theme == dark {
                    color: white;
                } @else {
                    color: black;
                }
            }
        """)
        assert "color: black;" in result
        assert "white" not in result

    def test_if_else_if(self) -> None:
        """@if false, @else if true matches second branch."""
        result = _compile("""
            $size: medium;
            h1 {
                @if $size == large {
                    font-size: 24px;
                } @else if $size == medium {
                    font-size: 16px;
                } @else {
                    font-size: 12px;
                }
            }
        """)
        assert "font-size: 16px;" in result
        assert "24px" not in result
        assert "12px" not in result


class TestForDirective:
    """Test @for loop expansion."""

    def test_for_through(self) -> None:
        """@for $i from 1 through 3 produces 3 iterations."""
        result = _compile("""
            h1 {
                @for $i from 1 through 3 {
                    color: red;
                }
            }
        """)
        assert result.count("color: red;") == 3

    def test_for_to(self) -> None:
        """@for $i from 1 to 3 produces 2 iterations (exclusive)."""
        result = _compile("""
            h1 {
                @for $i from 1 to 3 {
                    color: red;
                }
            }
        """)
        assert result.count("color: red;") == 2


class TestEachDirective:
    """Test @each loop expansion."""

    def test_simple_each(self) -> None:
        """@each iterates over values."""
        result = _compile("""
            h1 {
                @each $c in red, blue, green {
                    color: $c;
                }
            }
        """)
        assert "color: red;" in result
        assert "color: blue;" in result
        assert "color: green;" in result


# ===========================================================================
# Function Tests
# ===========================================================================


class TestFunctions:
    """Test @function definition and evaluation."""

    def test_simple_function(self) -> None:
        """@function with @return evaluates correctly."""
        result = _compile("""
            @function double($n) {
                @return $n * 2;
            }
            h1 { width: double(5); }
        """)
        assert "10" in result

    def test_function_with_dimension(self) -> None:
        """@function returning dimension value."""
        result = _compile("""
            @function spacing($n) {
                @return $n * 8px;
            }
            h1 { padding: spacing(2); }
        """)
        assert "16px" in result

    def test_function_with_default_param(self) -> None:
        """@function with default parameter, called with empty args.

        spacing() parses as FUNCTION + empty function_args + RPAREN.
        The transformer sees 0 args and uses the default value.
        """
        result = _compile("""
            @function spacing($n: 1) {
                @return $n * 8px;
            }
            h1 { padding: spacing(); }
        """)
        assert "8px" in result

    def test_function_with_conditional(self) -> None:
        """@function with @if inside body (positive branch)."""
        result = _compile("""
            @function check($n) {
                @if $n >= 0 {
                    @return $n;
                } @else {
                    @return 0;
                }
            }
            h1 { width: check(5); }
        """)
        assert "5" in result


class TestFunctionErrors:
    """Test error handling for functions."""

    def test_missing_return(self) -> None:
        with pytest.raises(MissingReturnError):
            _compile("""
                @function noop($n) {
                    $x: $n;
                }
                h1 { width: noop(5); }
            """)

    def test_wrong_function_arity(self) -> None:
        """Calling a function with wrong number of args raises WrongArityError."""
        with pytest.raises(WrongArityError):
            _compile("""
                @function double($n) { @return $n * 2; }
                h1 { width: double(1, 2); }
            """)


# ===========================================================================
# CSS Passthrough Tests
# ===========================================================================


class TestCSSPassthrough:
    """Test that plain CSS passes through unchanged."""

    def test_simple_rule(self) -> None:
        result = _compile("h1 { color: red; }")
        assert "h1" in result
        assert "color: red;" in result

    def test_media_query(self) -> None:
        result = _compile("@media (max-width: 768px) { h1 { color: red; } }")
        assert "@media" in result
        assert "color: red;" in result

    def test_multiple_rules(self) -> None:
        result = _compile("h1 { color: red; }\nh2 { color: blue; }")
        assert "red" in result
        assert "blue" in result

    def test_function_values(self) -> None:
        result = _compile("h1 { color: rgb(255, 0, 0); }")
        assert "rgb(" in result

    def test_nested_rule(self) -> None:
        result = _compile("div { & > p { color: red; } }")
        assert "color: red;" in result


# ===========================================================================
# @use Directive Tests
# ===========================================================================


class TestUseDirective:
    """Test @use directive handling."""

    def test_use_removed(self) -> None:
        """@use directives are removed from output."""
        result = _compile("""
            @use "colors";
            h1 { color: red; }
        """)
        assert "@use" not in result
        assert "color: red;" in result


# ===========================================================================
# Mixed Feature Tests
# ===========================================================================


class TestMixedFeatures:
    """Test combinations of Lattice features."""

    def test_variable_in_mixin(self) -> None:
        """Variables used inside mixin body."""
        result = _compile("""
            $primary: blue;
            @mixin theme() {
                color: $primary;
            }
            h1 { @include theme; }
        """)
        assert "color: blue;" in result

    def test_variable_and_rule(self) -> None:
        """Variable declaration + CSS rule."""
        result = _compile("""
            $size: 16px;
            h1 { font-size: $size; }
            h2 { font-size: 24px; }
        """)
        assert "font-size: 16px;" in result
        assert "font-size: 24px;" in result
