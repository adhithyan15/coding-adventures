"""Tests for the 2D box layout engine (cas_pretty_printer.box).

Test strategy
-------------
We test three layers:

1. **Primitive operations** — ``atom_box``, ``hbox``, ``vbox``, ``Box.pad_width``.
   These verify the core geometry without involving IR dispatch.

2. **Box builders** — ``_div_box``, ``_pow_box``, ``_sqrt_box`` directly,
   plus the ``_box`` dispatcher for each supported IR head.

3. **End-to-end via ``pretty(...)``** — using the public ``pretty()`` function
   with ``style="2d"`` to verify walker integration.

Unicode box-drawing characters used
-------------------------------------
- ``─`` (U+2500) LIGHT HORIZONTAL — fraction bars and sqrt overlines.
- ``│`` (U+2502) LIGHT VERTICAL — sqrt side walls.
- ``┌`` (U+250C) LIGHT DOWN AND RIGHT — sqrt top-left corner.
- ``┐`` (U+2510) LIGHT DOWN AND LEFT — sqrt top-right corner.
- ``√`` (U+221A) SQUARE ROOT — radical sign.
"""

from __future__ import annotations

import pytest
from symbolic_ir import (
    ADD,
    DIV,
    MUL,
    NEG,
    POW,
    SQRT,
    SUB,
    IRApply,
    IRFloat,
    IRInteger,
    IRRational,
    IRString,
    IRSymbol,
)

from cas_pretty_printer import MacsymaDialect, pretty
from cas_pretty_printer.box import atom_box, hbox, pretty_2d, vbox

# Shared dialect instance — re-used across all tests.
D = MacsymaDialect()

# Convenience: x and y as symbols.
_X = IRSymbol("x")
_Y = IRSymbol("y")


# ---------------------------------------------------------------------------
# Section 1: atom_box primitives
# ---------------------------------------------------------------------------


def test_atom_box_single_char() -> None:
    b = atom_box("x")
    assert b.height == 1
    assert b.baseline == 0
    assert b.width == 1
    assert b.lines == ["x"]


def test_atom_box_two_chars() -> None:
    b = atom_box("42")
    assert b.width == 2
    assert b.height == 1
    assert b.baseline == 0


def test_atom_box_renders_correctly() -> None:
    assert atom_box("hello").render() == "hello"


def test_atom_box_empty_string() -> None:
    b = atom_box("")
    assert b.width == 0
    assert b.height == 1


# ---------------------------------------------------------------------------
# Section 2: hbox horizontal composition
# ---------------------------------------------------------------------------


def test_hbox_two_atoms_no_sep() -> None:
    b = hbox([atom_box("a"), atom_box("b")])
    assert b.height == 1
    assert b.baseline == 0
    assert b.render() == "ab"


def test_hbox_two_atoms_with_sep() -> None:
    b = hbox([atom_box("a"), atom_box("b")], sep=" ")
    assert b.render() == "a b"


def test_hbox_three_atoms() -> None:
    b = hbox([atom_box("x"), atom_box("+"), atom_box("y")])
    assert b.render() == "x+y"


def test_hbox_empty_list() -> None:
    b = hbox([])
    assert b.height == 1
    assert b.render() == ""


def test_hbox_preserves_baseline() -> None:
    # All atoms have baseline 0, so hbox baseline should be 0.
    b = hbox([atom_box("a"), atom_box("b"), atom_box("c")])
    assert b.baseline == 0


# ---------------------------------------------------------------------------
# Section 3: vbox vertical composition
# ---------------------------------------------------------------------------


def test_vbox_two_rows() -> None:
    b = vbox([atom_box("a"), atom_box("b")])
    assert b.height == 2
    assert "a" in b.lines[0]
    assert "b" in b.lines[1]


def test_vbox_centres_narrower_box() -> None:
    wide = atom_box("wide")
    narrow = atom_box("x")
    b = vbox([wide, narrow])
    # x should be padded to width 4.
    assert b.width == 4
    assert b.height == 2


# ---------------------------------------------------------------------------
# Section 4: Box.pad_width
# ---------------------------------------------------------------------------


def test_pad_width_center() -> None:
    b = atom_box("x").pad_width(5, align="center")
    assert b.width == 5
    # "x" centred in 5 chars: 2 left spaces, 2 right spaces.
    assert b.lines[0] == "  x  "


def test_pad_width_left() -> None:
    b = atom_box("x").pad_width(4, align="left")
    assert b.lines[0] == "x   "


def test_pad_width_right() -> None:
    b = atom_box("x").pad_width(4, align="right")
    assert b.lines[0] == "   x"


def test_pad_width_no_change_if_already_wide() -> None:
    b = atom_box("hello")
    assert b.pad_width(3) is b  # no-op returns same object


# ---------------------------------------------------------------------------
# Section 5: Div (fraction) rendering
# ---------------------------------------------------------------------------


def test_div_renders_three_lines() -> None:
    """Div(x, y) must produce exactly 3 rows: num, bar, denom."""
    expr = IRApply(DIV, (_X, _Y))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    assert len(lines) == 3


def test_div_bar_contains_dashes() -> None:
    """The middle row of a fraction must be all ─ characters."""
    expr = IRApply(DIV, (_X, _Y))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    bar = lines[1].strip()
    assert all(c == "─" for c in bar)
    assert len(bar) >= 1


def test_div_bar_width_is_max_of_operands() -> None:
    """Bar width ≥ max(numerator width, denominator width)."""
    # Numerator "x" (width 1), denominator "y" (width 1) → bar ≥ 1.
    expr = IRApply(DIV, (_X, _Y))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    bar_width = len(lines[1].strip())
    assert bar_width >= 1


def test_div_numerator_centred() -> None:
    """Numerator and denominator appear in lines[0] and lines[2]."""
    expr = IRApply(DIV, (IRInteger(1), IRInteger(2)))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    assert "1" in lines[0]
    assert "2" in lines[2]


def test_div_via_pretty_function() -> None:
    """pretty(..., style="2d") for Div returns a multi-line string with ─."""
    expr = IRApply(DIV, (_X, _Y))
    result = pretty(expr, D, style="2d")
    assert "\n" in result
    assert "─" in result


# ---------------------------------------------------------------------------
# Section 6: Pow (superscript) rendering
# ---------------------------------------------------------------------------


def test_pow_renders_two_rows() -> None:
    """Pow(x, 2) should produce 2 rows: exponent on top, base on bottom."""
    expr = IRApply(POW, (_X, IRInteger(2)))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    assert len(lines) == 2


def test_pow_x_on_bottom_row() -> None:
    """The base symbol appears in the bottom row of the power box."""
    expr = IRApply(POW, (_X, IRInteger(2)))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    # Base is on the bottom row.
    assert "x" in lines[-1]


def test_pow_exponent_on_top_row() -> None:
    """The exponent appears in the top row of the power box."""
    expr = IRApply(POW, (_X, IRInteger(2)))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    assert "2" in lines[0]


# ---------------------------------------------------------------------------
# Section 7: Sqrt rendering
# ---------------------------------------------------------------------------


def test_sqrt_renders_two_rows() -> None:
    """Sqrt(x) should produce at least 2 rows (top border + content)."""
    expr = IRApply(SQRT, (_X,))
    result = pretty_2d(expr, D)
    lines = result.split("\n")
    assert len(lines) >= 2


def test_sqrt_contains_radical_sign() -> None:
    """The rendered sqrt must contain the √ character."""
    expr = IRApply(SQRT, (_X,))
    result = pretty_2d(expr, D)
    assert "√" in result


def test_sqrt_contains_overline_chars() -> None:
    """The top border of sqrt must contain ─ characters."""
    expr = IRApply(SQRT, (_X,))
    result = pretty_2d(expr, D)
    assert "─" in result


# ---------------------------------------------------------------------------
# Section 8: Add, Sub, Mul at baseline
# ---------------------------------------------------------------------------


def test_add_contains_plus() -> None:
    expr = IRApply(ADD, (_X, _Y))
    result = pretty(expr, D, style="2d")
    assert "+" in result


def test_sub_contains_minus() -> None:
    expr = IRApply(SUB, (_X, _Y))
    result = pretty(expr, D, style="2d")
    assert "-" in result


def test_mul_contains_star() -> None:
    expr = IRApply(MUL, (_X, _Y))
    result = pretty(expr, D, style="2d")
    assert "*" in result


# ---------------------------------------------------------------------------
# Section 9: Leaf nodes via pretty()
# ---------------------------------------------------------------------------


def test_integer_leaf() -> None:
    assert pretty(IRInteger(42), D, style="2d") == "42"


def test_symbol_leaf() -> None:
    assert pretty(_X, D, style="2d") == "x"


def test_float_leaf() -> None:
    result = pretty(IRFloat(3.14), D, style="2d")
    assert "3.14" in result


def test_rational_leaf() -> None:
    result = pretty(IRRational(1, 2), D, style="2d")
    assert "1" in result
    assert "2" in result


def test_string_leaf() -> None:
    result = pretty(IRString("hello"), D, style="2d")
    assert "hello" in result


# ---------------------------------------------------------------------------
# Section 10: Neg
# ---------------------------------------------------------------------------


def test_neg_integer() -> None:
    expr = IRApply(NEG, (IRInteger(2),))
    result = pretty(expr, D, style="2d")
    assert "-" in result
    assert "2" in result


# ---------------------------------------------------------------------------
# Section 11: Nested structures
# ---------------------------------------------------------------------------


def test_nested_fraction_more_than_3_lines() -> None:
    """Div(Add(x, 1), Div(y, 2)) should produce more than 3 rows."""
    inner_div = IRApply(DIV, (_Y, IRInteger(2)))
    outer_div = IRApply(DIV, (IRApply(ADD, (_X, IRInteger(1))), inner_div))
    result = pretty_2d(outer_div, D)
    lines = result.split("\n")
    assert len(lines) > 3


# ---------------------------------------------------------------------------
# Section 12: style parameter validation in walker
# ---------------------------------------------------------------------------


def test_linear_style_still_works() -> None:
    """Existing linear style should be unchanged."""
    result = pretty(IRInteger(5), D, style="linear")
    assert result == "5"


def test_2d_style_no_longer_raises() -> None:
    """style='2d' must not raise ValueError anymore."""
    result = pretty(_X, D, style="2d")
    assert result == "x"  # simple symbol is single-line


def test_3d_style_still_raises() -> None:
    """style='3d' (unknown) must still raise ValueError."""
    with pytest.raises(ValueError, match="unsupported style"):
        pretty(_X, D, style="3d")


def test_unknown_style_raises() -> None:
    """Any unknown style must raise ValueError."""
    with pytest.raises(ValueError):
        pretty(_X, D, style="ascii-art")


# ---------------------------------------------------------------------------
# Section 13: MNewton pretty-print name
# ---------------------------------------------------------------------------


def test_mnewton_function_name_in_linear() -> None:
    """The dialect maps MNewton → mnewton in function-call form."""
    f = IRApply(SUB, (_X, IRInteger(2)))
    expr = IRApply(IRSymbol("MNewton"), (f, _X, IRFloat(0.0)))
    result = pretty(expr, D, style="linear")
    assert "mnewton" in result


def test_mnewton_function_name_in_2d() -> None:
    """MNewton in 2D mode falls back to linear function-call (no special layout)."""
    f = IRApply(SUB, (_X, IRInteger(2)))
    expr = IRApply(IRSymbol("MNewton"), (f, _X, IRFloat(0.0)))
    result = pretty(expr, D, style="2d")
    assert "mnewton" in result
