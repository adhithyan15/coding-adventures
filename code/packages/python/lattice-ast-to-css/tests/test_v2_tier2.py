"""Tests for Lattice v2 Tier 2 features.

Tier 2 introduces new value types and built-in functions:

8. Maps — ordered key-value store with map-get, map-keys, etc.
9. Built-in color functions — lighten, darken, mix, complement, etc.
10. Built-in list/type/math functions — nth, length, type-of, math.div, etc.

These tests exercise the evaluator's built-in function registry and the
new LatticeMap value type directly.
"""

from __future__ import annotations

import pytest

from lattice_ast_to_css.errors import (
    RangeError,
    TypeErrorInExpression,
    ZeroDivisionInExpressionError,
)
from lattice_ast_to_css.evaluator import (
    BUILTIN_FUNCTIONS,
    LatticeColor,
    LatticeBool,
    LatticeDimension,
    LatticeIdent,
    LatticeList,
    LatticeMap,
    LatticeNull,
    LatticeNumber,
    LatticePercentage,
    LatticeString,
    is_truthy,
    value_to_css,
)
from lattice_ast_to_css.scope import ScopeChain


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _call(name: str, *args: object) -> object:
    """Call a built-in function by name with the given arguments."""
    scope = ScopeChain()
    return BUILTIN_FUNCTIONS[name](list(args), scope)


# ===========================================================================
# 8. Map Value Type Tests
# ===========================================================================


class TestLatticeMap:
    """Test the LatticeMap frozen dataclass and its methods."""

    def test_empty_map(self) -> None:
        """An empty map should have no items."""
        m = LatticeMap(())
        assert m.keys() == ()
        assert m.values() == ()
        assert m.get("anything") is None
        assert not m.has_key("anything")

    def test_basic_map(self) -> None:
        """A map with entries should support get/keys/values/has_key."""
        m = LatticeMap((
            ("primary", LatticeColor("#4a90d9")),
            ("secondary", LatticeColor("#7b68ee")),
        ))
        assert m.get("primary") == LatticeColor("#4a90d9")
        assert m.get("secondary") == LatticeColor("#7b68ee")
        assert m.get("tertiary") is None
        assert m.keys() == ("primary", "secondary")
        assert m.values() == (LatticeColor("#4a90d9"), LatticeColor("#7b68ee"))
        assert m.has_key("primary")
        assert not m.has_key("tertiary")

    def test_map_str(self) -> None:
        """Map string representation should be parenthesized key-value pairs."""
        m = LatticeMap((
            ("a", LatticeNumber(1.0)),
            ("b", LatticeNumber(2.0)),
        ))
        assert str(m) == "(a: 1, b: 2)"

    def test_map_is_truthy(self) -> None:
        """Maps should always be truthy, even when empty."""
        assert is_truthy(LatticeMap(()))
        assert is_truthy(LatticeMap((("a", LatticeNumber(1.0)),)))

    def test_map_frozen(self) -> None:
        """LatticeMap should be immutable (frozen dataclass)."""
        m = LatticeMap((("a", LatticeNumber(1.0)),))
        with pytest.raises(AttributeError):
            m.items = ()  # type: ignore[misc]

    def test_map_equality(self) -> None:
        """Two maps with same entries in same order should be equal."""
        m1 = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        m2 = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        assert m1 == m2

    def test_map_order_matters(self) -> None:
        """Maps with same entries in different order should NOT be equal."""
        m1 = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        m2 = LatticeMap((("b", LatticeNumber(2.0)), ("a", LatticeNumber(1.0))))
        assert m1 != m2


# ===========================================================================
# Map Built-in Functions
# ===========================================================================


class TestMapFunctions:
    """Test map-get, map-keys, map-values, map-has-key, map-merge, map-remove."""

    def _sample_map(self) -> LatticeMap:
        return LatticeMap((
            ("primary", LatticeColor("#4a90d9")),
            ("secondary", LatticeColor("#7b68ee")),
            ("background", LatticeColor("#ffffff")),
        ))

    def test_map_get_found(self) -> None:
        """map-get should return the value for an existing key."""
        result = _call("map-get", self._sample_map(), LatticeIdent("primary"))
        assert result == LatticeColor("#4a90d9")

    def test_map_get_not_found(self) -> None:
        """map-get should return null for a missing key."""
        result = _call("map-get", self._sample_map(), LatticeIdent("tertiary"))
        assert isinstance(result, LatticeNull)

    def test_map_get_wrong_type(self) -> None:
        """map-get on non-map should raise TypeError."""
        with pytest.raises(TypeErrorInExpression):
            _call("map-get", LatticeNumber(42.0), LatticeIdent("key"))

    def test_map_keys(self) -> None:
        """map-keys should return all keys as a list."""
        result = _call("map-keys", self._sample_map())
        assert isinstance(result, LatticeList)
        assert len(result.items) == 3
        assert str(result.items[0]) == "primary"

    def test_map_values(self) -> None:
        """map-values should return all values as a list."""
        result = _call("map-values", self._sample_map())
        assert isinstance(result, LatticeList)
        assert len(result.items) == 3
        assert result.items[0] == LatticeColor("#4a90d9")

    def test_map_has_key_true(self) -> None:
        """map-has-key should return true for existing key."""
        result = _call("map-has-key", self._sample_map(), LatticeIdent("primary"))
        assert result == LatticeBool(True)

    def test_map_has_key_false(self) -> None:
        """map-has-key should return false for missing key."""
        result = _call("map-has-key", self._sample_map(), LatticeIdent("tertiary"))
        assert result == LatticeBool(False)

    def test_map_merge(self) -> None:
        """map-merge should combine two maps, second wins on conflicts."""
        m1 = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        m2 = LatticeMap((("b", LatticeNumber(99.0)), ("c", LatticeNumber(3.0))))
        result = _call("map-merge", m1, m2)
        assert isinstance(result, LatticeMap)
        assert result.get("a") == LatticeNumber(1.0)
        assert result.get("b") == LatticeNumber(99.0)  # m2 wins
        assert result.get("c") == LatticeNumber(3.0)

    def test_map_remove(self) -> None:
        """map-remove should return a new map without specified keys."""
        m = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0)), ("c", LatticeNumber(3.0))))
        result = _call("map-remove", m, LatticeIdent("b"))
        assert isinstance(result, LatticeMap)
        assert result.get("a") == LatticeNumber(1.0)
        assert result.get("b") is None
        assert result.get("c") == LatticeNumber(3.0)

    def test_map_remove_multiple_keys(self) -> None:
        """map-remove should handle multiple keys."""
        m = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0)), ("c", LatticeNumber(3.0))))
        result = _call("map-remove", m, LatticeIdent("a"), LatticeIdent("c"))
        assert isinstance(result, LatticeMap)
        assert len(result.items) == 1
        assert result.get("b") == LatticeNumber(2.0)


# ===========================================================================
# 9. Color Function Tests
# ===========================================================================


class TestColorFunctions:
    """Test built-in color manipulation functions.

    Color functions operate on hex colors and return hex colors (or rgba
    for semi-transparent results).
    """

    def test_color_to_rgb(self) -> None:
        """LatticeColor.to_rgb should parse hex colors correctly."""
        # 6-digit hex
        c = LatticeColor("#4a90d9")
        r, g, b, a = c.to_rgb()
        assert r == 74
        assert g == 144
        assert b == 217
        assert a == 1.0

    def test_color_to_rgb_shorthand(self) -> None:
        """to_rgb should handle 3-digit shorthand."""
        c = LatticeColor("#f00")
        r, g, b, a = c.to_rgb()
        assert r == 255
        assert g == 0
        assert b == 0

    def test_color_to_hsl(self) -> None:
        """to_hsl should convert correctly."""
        # Pure red = hue 0, saturation 100, lightness 50
        c = LatticeColor("#ff0000")
        h, s, l, a = c.to_hsl()
        assert abs(h - 0.0) < 1.0
        assert abs(s - 100.0) < 1.0
        assert abs(l - 50.0) < 1.0

    def test_color_from_rgb(self) -> None:
        """from_rgb should produce valid hex colors."""
        c = LatticeColor.from_rgb(255, 0, 0)
        assert c.value == "#ff0000"

    def test_color_from_rgb_alpha(self) -> None:
        """from_rgb with alpha should produce rgba()."""
        c = LatticeColor.from_rgb(255, 0, 0, 0.5)
        assert "rgba" in c.value
        assert "0.5" in c.value

    def test_color_from_hsl(self) -> None:
        """from_hsl should round-trip correctly."""
        # Pure red: h=0, s=100, l=50
        c = LatticeColor.from_hsl(0, 100, 50)
        r, g, b, a = c.to_rgb()
        assert r == 255
        assert g == 0
        assert b == 0

    def test_lighten(self) -> None:
        """lighten should increase lightness."""
        dark = LatticeColor("#333333")
        result = _call("lighten", dark, LatticePercentage(20.0))
        assert isinstance(result, LatticeColor)
        # The lightened color should be brighter (higher RGB values)
        r1, g1, b1, _ = dark.to_rgb()
        r2, g2, b2, _ = result.to_rgb()
        assert r2 > r1
        assert g2 > g1
        assert b2 > b1

    def test_darken(self) -> None:
        """darken should decrease lightness."""
        light = LatticeColor("#cccccc")
        result = _call("darken", light, LatticePercentage(20.0))
        assert isinstance(result, LatticeColor)
        r1, g1, b1, _ = light.to_rgb()
        r2, g2, b2, _ = result.to_rgb()
        assert r2 < r1

    def test_complement(self) -> None:
        """complement should rotate hue by 180 degrees."""
        red = LatticeColor("#ff0000")
        result = _call("complement", red)
        assert isinstance(result, LatticeColor)
        # Complement of red should be cyan-ish
        r, g, b, _ = result.to_rgb()
        assert r == 0  # No red in complement of pure red
        assert g > 200  # Cyan has high green
        assert b > 200  # Cyan has high blue

    def test_mix_equal_weight(self) -> None:
        """mix with 50% weight should blend evenly."""
        red = LatticeColor("#ff0000")
        blue = LatticeColor("#0000ff")
        result = _call("mix", red, blue, LatticePercentage(50.0))
        assert isinstance(result, LatticeColor)
        r, g, b, _ = result.to_rgb()
        # Even mix of red and blue → purple
        assert 120 <= r <= 135  # ~128
        assert g == 0
        assert 120 <= b <= 135

    def test_mix_weighted(self) -> None:
        """mix with 100% weight should return first color."""
        red = LatticeColor("#ff0000")
        blue = LatticeColor("#0000ff")
        result = _call("mix", red, blue, LatticePercentage(100.0))
        r, g, b, _ = result.to_rgb()
        assert r == 255
        assert b == 0

    def test_rgba_with_color(self) -> None:
        """rgba($color, $alpha) should set the alpha channel."""
        result = _call("rgba", LatticeColor("#ff0000"), LatticeNumber(0.5))
        assert isinstance(result, LatticeColor)
        assert "rgba" in result.value
        assert "0.5" in result.value

    def test_red_channel(self) -> None:
        """red() should extract red channel."""
        result = _call("red", LatticeColor("#4a90d9"))
        assert isinstance(result, LatticeNumber)
        assert result.value == 74.0

    def test_green_channel(self) -> None:
        """green() should extract green channel."""
        result = _call("green", LatticeColor("#4a90d9"))
        assert isinstance(result, LatticeNumber)
        assert result.value == 144.0

    def test_blue_channel(self) -> None:
        """blue() should extract blue channel."""
        result = _call("blue", LatticeColor("#4a90d9"))
        assert isinstance(result, LatticeNumber)
        assert result.value == 217.0

    def test_hue(self) -> None:
        """hue() should extract hue in degrees."""
        result = _call("hue", LatticeColor("#ff0000"))
        assert isinstance(result, LatticeDimension)
        assert result.unit == "deg"
        assert result.value == 0.0

    def test_saturation(self) -> None:
        """saturation() should extract saturation percentage."""
        result = _call("saturation", LatticeColor("#ff0000"))
        assert isinstance(result, LatticePercentage)
        assert result.value == 100.0

    def test_lightness(self) -> None:
        """lightness() should extract lightness percentage."""
        result = _call("lightness", LatticeColor("#ff0000"))
        assert isinstance(result, LatticePercentage)
        assert result.value == 50.0

    def test_lighten_invalid_type(self) -> None:
        """lighten on non-color should raise TypeError."""
        with pytest.raises(TypeErrorInExpression):
            _call("lighten", LatticeNumber(42.0), LatticePercentage(10.0))

    def test_lighten_out_of_range(self) -> None:
        """lighten with amount > 100% should raise RangeError."""
        with pytest.raises(RangeError):
            _call("lighten", LatticeColor("#000"), LatticePercentage(150.0))

    def test_saturate_fn(self) -> None:
        """saturate should increase saturation."""
        gray = LatticeColor("#808080")
        result = _call("saturate", gray, LatticePercentage(50.0))
        assert isinstance(result, LatticeColor)

    def test_desaturate(self) -> None:
        """desaturate should decrease saturation."""
        bright = LatticeColor("#ff0000")
        result = _call("desaturate", bright, LatticePercentage(50.0))
        assert isinstance(result, LatticeColor)
        _, s, _, _ = result.to_hsl()
        assert s < 100.0

    def test_adjust_hue(self) -> None:
        """adjust-hue should rotate hue."""
        red = LatticeColor("#ff0000")
        result = _call("adjust-hue", red, LatticeNumber(120.0))
        assert isinstance(result, LatticeColor)
        # Red + 120deg hue → green
        r, g, b, _ = result.to_rgb()
        assert g > r  # Should be greenish

    def test_pure_black_lighten(self) -> None:
        """lighten pure black should produce gray."""
        result = _call("lighten", LatticeColor("#000000"), LatticePercentage(50.0))
        r, g, b, _ = result.to_rgb()
        assert r > 0
        assert g > 0
        assert b > 0

    def test_pure_white_darken(self) -> None:
        """darken pure white should produce gray."""
        result = _call("darken", LatticeColor("#ffffff"), LatticePercentage(50.0))
        r, g, b, _ = result.to_rgb()
        assert r < 255
        assert g < 255
        assert b < 255


# ===========================================================================
# 10. List/Type/Math Function Tests
# ===========================================================================


class TestListFunctions:
    """Test built-in list functions: nth, length, join, append, index."""

    def test_nth_first(self) -> None:
        """nth should return the first element (1-indexed)."""
        lst = LatticeList((LatticeDimension(10.0, "px"), LatticeDimension(20.0, "px")))
        result = _call("nth", lst, LatticeNumber(1.0))
        assert result == LatticeDimension(10.0, "px")

    def test_nth_last(self) -> None:
        """nth should return the last element."""
        lst = LatticeList((LatticeIdent("a"), LatticeIdent("b"), LatticeIdent("c")))
        result = _call("nth", lst, LatticeNumber(3.0))
        assert result == LatticeIdent("c")

    def test_nth_out_of_bounds(self) -> None:
        """nth with index > length should raise RangeError."""
        lst = LatticeList((LatticeNumber(1.0),))
        with pytest.raises(RangeError):
            _call("nth", lst, LatticeNumber(5.0))

    def test_nth_zero_index(self) -> None:
        """nth with index 0 should raise RangeError."""
        lst = LatticeList((LatticeNumber(1.0),))
        with pytest.raises(RangeError):
            _call("nth", lst, LatticeNumber(0.0))

    def test_nth_negative_index(self) -> None:
        """nth with negative index should raise RangeError."""
        lst = LatticeList((LatticeNumber(1.0),))
        with pytest.raises(RangeError):
            _call("nth", lst, LatticeNumber(-1.0))

    def test_length_list(self) -> None:
        """length of a list should return item count."""
        lst = LatticeList((LatticeNumber(1.0), LatticeNumber(2.0), LatticeNumber(3.0)))
        result = _call("length", lst)
        assert result == LatticeNumber(3.0)

    def test_length_map(self) -> None:
        """length of a map should return entry count."""
        m = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        result = _call("length", m)
        assert result == LatticeNumber(2.0)

    def test_length_single_value(self) -> None:
        """length of a single value should return 1."""
        result = _call("length", LatticeNumber(42.0))
        assert result == LatticeNumber(1.0)

    def test_join(self) -> None:
        """join should concatenate two lists."""
        l1 = LatticeList((LatticeIdent("a"), LatticeIdent("b")))
        l2 = LatticeList((LatticeIdent("c"), LatticeIdent("d")))
        result = _call("join", l1, l2)
        assert isinstance(result, LatticeList)
        assert len(result.items) == 4

    def test_append(self) -> None:
        """append should add value to end of list."""
        lst = LatticeList((LatticeIdent("a"), LatticeIdent("b")))
        result = _call("append", lst, LatticeIdent("c"))
        assert isinstance(result, LatticeList)
        assert len(result.items) == 3
        assert result.items[2] == LatticeIdent("c")

    def test_index_found(self) -> None:
        """index should return 1-based position when found."""
        lst = LatticeList((LatticeIdent("a"), LatticeIdent("b"), LatticeIdent("c")))
        result = _call("index", lst, LatticeIdent("b"))
        assert result == LatticeNumber(2.0)

    def test_index_not_found(self) -> None:
        """index should return null when value not found."""
        lst = LatticeList((LatticeIdent("a"), LatticeIdent("b")))
        result = _call("index", lst, LatticeIdent("z"))
        assert isinstance(result, LatticeNull)


class TestTypeFunctions:
    """Test built-in type introspection functions."""

    def test_type_of_number(self) -> None:
        result = _call("type-of", LatticeNumber(42.0))
        assert result == LatticeString("number")

    def test_type_of_dimension(self) -> None:
        result = _call("type-of", LatticeDimension(16.0, "px"))
        assert result == LatticeString("number")

    def test_type_of_string(self) -> None:
        result = _call("type-of", LatticeString("hello"))
        assert result == LatticeString("string")

    def test_type_of_color(self) -> None:
        result = _call("type-of", LatticeColor("#ff0000"))
        assert result == LatticeString("color")

    def test_type_of_bool(self) -> None:
        result = _call("type-of", LatticeBool(True))
        assert result == LatticeString("bool")

    def test_type_of_null(self) -> None:
        result = _call("type-of", LatticeNull())
        assert result == LatticeString("null")

    def test_type_of_list(self) -> None:
        result = _call("type-of", LatticeList((LatticeNumber(1.0),)))
        assert result == LatticeString("list")

    def test_type_of_map(self) -> None:
        result = _call("type-of", LatticeMap((("a", LatticeNumber(1.0)),)))
        assert result == LatticeString("map")

    def test_unit_px(self) -> None:
        result = _call("unit", LatticeDimension(16.0, "px"))
        assert result == LatticeString("px")

    def test_unit_percentage(self) -> None:
        result = _call("unit", LatticePercentage(50.0))
        assert result == LatticeString("%")

    def test_unit_unitless(self) -> None:
        result = _call("unit", LatticeNumber(42.0))
        assert result == LatticeString("")

    def test_unit_non_number(self) -> None:
        with pytest.raises(TypeErrorInExpression):
            _call("unit", LatticeString("hello"))

    def test_unitless_true(self) -> None:
        result = _call("unitless", LatticeNumber(42.0))
        assert result == LatticeBool(True)

    def test_unitless_false(self) -> None:
        result = _call("unitless", LatticeDimension(16.0, "px"))
        assert result == LatticeBool(False)

    def test_comparable_same_unit(self) -> None:
        result = _call("comparable", LatticeDimension(1.0, "px"), LatticeDimension(2.0, "px"))
        assert result == LatticeBool(True)

    def test_comparable_different_unit(self) -> None:
        result = _call("comparable", LatticeDimension(1.0, "px"), LatticeDimension(2.0, "em"))
        assert result == LatticeBool(False)

    def test_comparable_number_and_dimension(self) -> None:
        result = _call("comparable", LatticeNumber(1.0), LatticeDimension(2.0, "px"))
        assert result == LatticeBool(True)


class TestMathFunctions:
    """Test built-in math functions: math.div, math.floor, math.ceil, etc."""

    def test_div_numbers(self) -> None:
        """math.div should divide two numbers."""
        result = _call("math.div", LatticeNumber(100.0), LatticeNumber(3.0))
        assert isinstance(result, LatticeNumber)
        assert abs(result.value - 33.333333) < 0.001

    def test_div_dimension_by_number(self) -> None:
        """math.div should handle dimension / number → dimension."""
        result = _call("math.div", LatticeDimension(100.0, "px"), LatticeNumber(3.0))
        assert isinstance(result, LatticeDimension)
        assert result.unit == "px"
        assert abs(result.value - 33.333333) < 0.001

    def test_div_dimension_by_same_unit(self) -> None:
        """math.div with same units should cancel → number."""
        result = _call("math.div", LatticeDimension(100.0, "px"), LatticeDimension(50.0, "px"))
        assert isinstance(result, LatticeNumber)
        assert result.value == 2.0

    def test_div_by_zero(self) -> None:
        """math.div by zero should raise ZeroDivisionError."""
        with pytest.raises(ZeroDivisionInExpressionError):
            _call("math.div", LatticeNumber(100.0), LatticeNumber(0.0))

    def test_div_percentage(self) -> None:
        """math.div with percentage / number → percentage."""
        result = _call("math.div", LatticePercentage(100.0), LatticeNumber(3.0))
        assert isinstance(result, LatticePercentage)

    def test_floor(self) -> None:
        """math.floor should round down."""
        result = _call("math.floor", LatticeNumber(3.7))
        assert result == LatticeNumber(3.0)

    def test_floor_negative(self) -> None:
        """math.floor on negative should round toward negative infinity."""
        result = _call("math.floor", LatticeNumber(-3.2))
        assert result == LatticeNumber(-4.0)

    def test_floor_dimension(self) -> None:
        """math.floor should preserve units."""
        result = _call("math.floor", LatticeDimension(3.7, "px"))
        assert isinstance(result, LatticeDimension)
        assert result.value == 3.0
        assert result.unit == "px"

    def test_ceil(self) -> None:
        """math.ceil should round up."""
        result = _call("math.ceil", LatticeNumber(3.2))
        assert result == LatticeNumber(4.0)

    def test_ceil_negative(self) -> None:
        """math.ceil on negative should round toward zero."""
        result = _call("math.ceil", LatticeNumber(-3.7))
        assert result == LatticeNumber(-3.0)

    def test_round(self) -> None:
        """math.round should round to nearest integer."""
        result = _call("math.round", LatticeNumber(3.5))
        assert result == LatticeNumber(4.0)

    def test_round_down(self) -> None:
        """math.round should round down when < .5."""
        result = _call("math.round", LatticeNumber(3.4))
        assert result == LatticeNumber(3.0)

    def test_round_dimension(self) -> None:
        """math.round should preserve units."""
        result = _call("math.round", LatticeDimension(3.7, "px"))
        assert isinstance(result, LatticeDimension)
        assert result.value == 4.0

    def test_abs_positive(self) -> None:
        """math.abs of positive should be unchanged."""
        result = _call("math.abs", LatticeNumber(5.0))
        assert result == LatticeNumber(5.0)

    def test_abs_negative(self) -> None:
        """math.abs of negative should be positive."""
        result = _call("math.abs", LatticeNumber(-5.0))
        assert result == LatticeNumber(5.0)

    def test_abs_zero(self) -> None:
        """math.abs of zero should be zero."""
        result = _call("math.abs", LatticeNumber(0.0))
        assert result == LatticeNumber(0.0)

    def test_abs_dimension(self) -> None:
        """math.abs should preserve units."""
        result = _call("math.abs", LatticeDimension(-5.0, "px"))
        assert isinstance(result, LatticeDimension)
        assert result.value == 5.0
        assert result.unit == "px"

    def test_min_single(self) -> None:
        """math.min with single arg should return it."""
        result = _call("math.min", LatticeNumber(5.0))
        assert result == LatticeNumber(5.0)

    def test_min_multiple(self) -> None:
        """math.min should return the smallest value."""
        result = _call("math.min",
                       LatticeDimension(10.0, "px"),
                       LatticeDimension(5.0, "px"),
                       LatticeDimension(20.0, "px"))
        assert result == LatticeDimension(5.0, "px")

    def test_max_multiple(self) -> None:
        """math.max should return the largest value."""
        result = _call("math.max",
                       LatticeDimension(10.0, "px"),
                       LatticeDimension(5.0, "px"),
                       LatticeDimension(20.0, "px"))
        assert result == LatticeDimension(20.0, "px")

    def test_floor_already_integer(self) -> None:
        """math.floor on integer should return same value."""
        result = _call("math.floor", LatticeNumber(5.0))
        assert result == LatticeNumber(5.0)

    def test_ceil_already_integer(self) -> None:
        """math.ceil on integer should return same value."""
        result = _call("math.ceil", LatticeNumber(5.0))
        assert result == LatticeNumber(5.0)


# ===========================================================================
# Error Type Tests
# ===========================================================================


class TestV2ErrorTypes:
    """Test new v2 error types."""

    def test_range_error(self) -> None:
        """RangeError should carry the message."""
        err = RangeError("Index 5 out of bounds for list of length 3")
        assert "Index 5" in str(err)

    def test_zero_division_error(self) -> None:
        """ZeroDivisionInExpressionError should have standard message."""
        err = ZeroDivisionInExpressionError()
        assert "Division by zero" in str(err)

    def test_range_error_is_lattice_error(self) -> None:
        """RangeError should inherit from LatticeError."""
        from lattice_ast_to_css.errors import LatticeError
        assert isinstance(RangeError("test"), LatticeError)

    def test_zero_division_is_lattice_error(self) -> None:
        """ZeroDivisionInExpressionError should inherit from LatticeError."""
        from lattice_ast_to_css.errors import LatticeError
        assert isinstance(ZeroDivisionInExpressionError(), LatticeError)


# ===========================================================================
# Integration: Features Combined
# ===========================================================================


class TestTier2Integration:
    """Test interactions between Tier 2 features."""

    def test_all_builtin_functions_registered(self) -> None:
        """All expected built-in functions should be in the registry."""
        expected = [
            "map-get", "map-keys", "map-values", "map-has-key",
            "map-merge", "map-remove",
            "lighten", "darken", "saturate", "desaturate",
            "adjust-hue", "complement", "mix", "rgba",
            "red", "green", "blue", "hue", "saturation", "lightness",
            "nth", "length", "join", "append", "index",
            "type-of", "unit", "unitless", "comparable",
            "math.div", "math.floor", "math.ceil", "math.round",
            "math.abs", "math.min", "math.max",
        ]
        for name in expected:
            assert name in BUILTIN_FUNCTIONS, f"Missing built-in: {name}"

    def test_map_with_color_values(self) -> None:
        """Maps should work with color values as entries."""
        m = LatticeMap((
            ("primary", LatticeColor("#4a90d9")),
            ("danger", LatticeColor("#ff0000")),
        ))
        result = _call("map-get", m, LatticeIdent("danger"))
        assert result == LatticeColor("#ff0000")
        lightened = _call("lighten", result, LatticePercentage(20.0))
        assert isinstance(lightened, LatticeColor)

    def test_length_of_map_keys(self) -> None:
        """length of map-keys result should match map length."""
        m = LatticeMap((("a", LatticeNumber(1.0)), ("b", LatticeNumber(2.0))))
        keys = _call("map-keys", m)
        length = _call("length", keys)
        assert length == LatticeNumber(2.0)

    def test_type_of_map_get_result(self) -> None:
        """type-of should work on map-get results."""
        m = LatticeMap((("size", LatticeDimension(16.0, "px")),))
        val = _call("map-get", m, LatticeIdent("size"))
        t = _call("type-of", val)
        assert t == LatticeString("number")
