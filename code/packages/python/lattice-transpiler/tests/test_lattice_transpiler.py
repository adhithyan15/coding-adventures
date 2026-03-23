"""Integration tests for the Lattice transpiler.

These tests exercise the full pipeline: Lattice source → CSS output.
They verify that variables, mixins, control flow, functions, and plain
CSS all transpile correctly through the entire stack.
"""

from __future__ import annotations

import pytest

from lattice_transpiler import __version__, transpile_lattice
from lattice_ast_to_css.errors import (
    LatticeError,
    UndefinedMixinError,
    UndefinedVariableError,
)


# ===========================================================================
# Version and Import
# ===========================================================================


class TestVersion:
    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"

    def test_transpile_lattice_is_callable(self) -> None:
        assert callable(transpile_lattice)


# ===========================================================================
# Variable Tests
# ===========================================================================


class TestVariables:
    def test_simple_variable(self) -> None:
        css = transpile_lattice("$color: red; h1 { color: $color; }")
        assert "color: red;" in css
        assert "$" not in css

    def test_hash_variable(self) -> None:
        css = transpile_lattice("$bg: #4a90d9; h1 { background: $bg; }")
        assert "#4a90d9" in css

    def test_dimension_variable(self) -> None:
        css = transpile_lattice("$size: 16px; h1 { font-size: $size; }")
        assert "16px" in css

    def test_multiple_variables(self) -> None:
        css = transpile_lattice("""
            $fg: white;
            $bg: black;
            h1 { color: $fg; background: $bg; }
        """)
        assert "color: white;" in css
        assert "background: black;" in css


# ===========================================================================
# Mixin Tests
# ===========================================================================


class TestMixins:
    def test_parameterless_mixin(self) -> None:
        css = transpile_lattice("""
            @mixin bold() { font-weight: bold; }
            h1 { @include bold; }
        """)
        assert "font-weight: bold;" in css
        assert "@mixin" not in css
        assert "@include" not in css

    def test_mixin_with_param(self) -> None:
        css = transpile_lattice("""
            @mixin colored($c) { color: $c; }
            h1 { @include colored(red); }
        """)
        assert "color: red;" in css

    def test_mixin_with_defaults(self) -> None:
        css = transpile_lattice("""
            @mixin themed($fg: white) { color: $fg; }
            h1 { @include themed; }
        """)
        assert "color: white;" in css

    def test_mixin_reuse(self) -> None:
        css = transpile_lattice("""
            @mixin bold() { font-weight: bold; }
            h1 { @include bold; }
            h2 { @include bold; }
            h3 { @include bold; }
        """)
        assert css.count("font-weight: bold;") == 3


# ===========================================================================
# Control Flow Tests
# ===========================================================================


class TestControlFlow:
    def test_if_true(self) -> None:
        css = transpile_lattice("""
            $dark: true;
            h1 { @if $dark { color: white; } }
        """)
        assert "color: white;" in css

    def test_if_false(self) -> None:
        css = transpile_lattice("""
            $dark: false;
            h1 { @if $dark { color: white; } }
        """)
        assert "white" not in css

    def test_if_else(self) -> None:
        css = transpile_lattice("""
            $theme: light;
            h1 {
                @if $theme == dark { color: white; }
                @else { color: black; }
            }
        """)
        assert "color: black;" in css

    def test_for_loop(self) -> None:
        css = transpile_lattice("""
            h1 { @for $i from 1 through 3 { font-size: 16px; } }
        """)
        assert css.count("font-size: 16px;") == 3

    def test_each_loop(self) -> None:
        css = transpile_lattice("""
            h1 {
                @each $c in red, blue, green {
                    color: $c;
                }
            }
        """)
        assert "color: red;" in css
        assert "color: blue;" in css
        assert "color: green;" in css


# ===========================================================================
# Function Tests
# ===========================================================================


class TestFunctions:
    def test_simple_function(self) -> None:
        css = transpile_lattice("""
            @function double($n) { @return $n * 2; }
            h1 { width: double(5); }
        """)
        assert "10" in css

    def test_function_with_dimension(self) -> None:
        css = transpile_lattice("""
            @function spacing($n) { @return $n * 8px; }
            h1 { padding: spacing(2); }
        """)
        assert "16px" in css


# ===========================================================================
# CSS Passthrough Tests
# ===========================================================================


class TestCSSPassthrough:
    def test_simple_css(self) -> None:
        css = transpile_lattice("h1 { color: red; }")
        assert "h1" in css
        assert "color: red;" in css

    def test_media_query(self) -> None:
        css = transpile_lattice("""
            @media (max-width: 768px) {
                h1 { color: red; }
            }
        """)
        assert "@media" in css
        assert "color: red;" in css

    def test_function_values(self) -> None:
        css = transpile_lattice("h1 { color: rgb(255, 0, 0); }")
        assert "rgb(" in css

    def test_empty_source(self) -> None:
        css = transpile_lattice("")
        assert css == ""


# ===========================================================================
# Formatting Tests
# ===========================================================================


class TestFormatting:
    def test_pretty_print(self) -> None:
        css = transpile_lattice("h1 { color: red; }")
        assert "\n" in css
        assert "  " in css

    def test_minified(self) -> None:
        css = transpile_lattice("h1 { color: red; }", minified=True)
        assert "\n" not in css.strip()

    def test_custom_indent(self) -> None:
        css = transpile_lattice("h1 { color: red; }", indent="    ")
        assert "    " in css


# ===========================================================================
# Error Tests
# ===========================================================================


class TestErrors:
    def test_undefined_variable(self) -> None:
        with pytest.raises(UndefinedVariableError):
            transpile_lattice("h1 { color: $nope; }")

    def test_undefined_mixin(self) -> None:
        with pytest.raises(UndefinedMixinError):
            transpile_lattice("h1 { @include nope; }")

    def test_all_errors_are_lattice_error(self) -> None:
        with pytest.raises(LatticeError):
            transpile_lattice("h1 { color: $undefined; }")


# ===========================================================================
# Stress Tests
# ===========================================================================


class TestStressTests:
    def test_variable_in_mixin_in_media(self) -> None:
        """Variable → mixin → @media — scope chain through expansion."""
        css = transpile_lattice("""
            $primary: blue;
            @mixin theme() { color: $primary; }
            @media (max-width: 768px) {
                h1 { @include theme; }
            }
        """)
        assert "color: blue;" in css
        assert "@media" in css

    def test_realistic_lattice_file(self) -> None:
        """A realistic Lattice file exercising multiple features."""
        css = transpile_lattice("""
            $primary: #4a90d9;
            $base-size: 16px;

            @mixin button($bg) {
                background: $bg;
                padding: 8px 16px;
            }

            @function double($n) {
                @return $n * 2;
            }

            h1 {
                font-size: $base-size;
                color: $primary;
            }

            .btn {
                @include button($primary);
            }

            .container {
                width: double(400px);
            }
        """)
        assert "#4a90d9" in css
        assert "16px" in css
        assert "background:" in css
        assert "padding:" in css
        assert "800px" in css
        # No Lattice constructs in output
        assert "$" not in css
        assert "@mixin" not in css
        assert "@include" not in css
        assert "@function" not in css
        assert "@return" not in css
