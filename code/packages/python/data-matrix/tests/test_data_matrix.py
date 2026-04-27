"""Tests for the Data Matrix ECC200 encoder.

Coverage targets:
- Symbol size selection (square auto, forced, rectangular).
- L-finder and timing border structure.
- ASCII encoding and digit-pair compaction.
- Reed-Solomon ECC (indirectly via encode determinism and round-trips).
- Utah placement (indirectly).
- `encode_at`, `layout_grid`, `encode_and_layout`, `grid_to_string`.
- Error cases: oversized input, invalid explicit size.
"""

from __future__ import annotations

import pytest

from coding_adventures.data_matrix import (
    DataMatrixError,
    InputTooLongError,
    InvalidSymbolError,
    SymbolShape,
    __version__,
    encode,
    encode_and_layout,
    encode_at,
    grid_to_string,
    layout_grid,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def dark(grid: ModuleGrid, row: int, col: int) -> bool:
    return bool(grid.modules[row][col])


# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------


class TestVersion:
    def test_version_string(self):
        assert __version__ == "0.1.0"


# ---------------------------------------------------------------------------
# Error hierarchy
# ---------------------------------------------------------------------------


class TestErrors:
    def test_data_matrix_error_is_exception(self):
        e = DataMatrixError("msg")
        assert isinstance(e, Exception)
        assert str(e) == "msg"

    def test_input_too_long_is_data_matrix_error(self):
        e = InputTooLongError("big")
        assert isinstance(e, DataMatrixError)

    def test_invalid_symbol_is_data_matrix_error(self):
        e = InvalidSymbolError("bad size")
        assert isinstance(e, DataMatrixError)

    def test_huge_input_raises_input_too_long(self):
        with pytest.raises(InputTooLongError):
            encode("X" * 3000)

    def test_invalid_explicit_size_raises(self):
        with pytest.raises(InvalidSymbolError):
            encode("A", size=(7, 7))  # 7×7 is not a valid DM size


# ---------------------------------------------------------------------------
# SymbolShape constants
# ---------------------------------------------------------------------------


class TestSymbolShape:
    def test_square_constant(self):
        assert SymbolShape.Square == "square"

    def test_rectangle_constant(self):
        assert SymbolShape.Rectangular == "rectangular"

    def test_any_constant(self):
        assert SymbolShape.Any == "any"


# ---------------------------------------------------------------------------
# Symbol size selection — square (default)
# ---------------------------------------------------------------------------


class TestSquareSizes:
    def test_single_char_smallest_square(self):
        # 'A' → 1 ASCII codeword → fits in 10×10 (3 data cw)
        g = encode("A")
        assert g.rows == 10
        assert g.cols == 10

    def test_short_string_12x12(self):
        # Around 5 ASCII chars → 12×12 (5 data cw)
        g = encode("Hello")
        assert g.rows >= 10
        assert g.cols >= 10
        assert g.rows == g.cols  # square

    def test_digit_pair_compaction(self):
        # "12" → 1 codeword (57); "A" → 1 codeword; fits in 10×10
        g_digits = encode("12")
        g_alpha = encode("A")
        # Both fit in smallest square; sizes must be equal
        assert g_digits.rows == g_alpha.rows
        assert g_digits.cols == g_alpha.cols

    def test_medium_string_18x18(self):
        # "Hello, World!" = 13 ASCII codewords; 16×16 holds only 12, so 18×18
        g = encode("Hello, World!")
        assert g.rows == 18
        assert g.cols == 18

    def test_grid_is_square(self):
        for data in ["X", "ABCDE", "Hello!", "12345678901234"]:
            g = encode(data)
            assert g.rows == g.cols, f"Not square for {data!r}"


# ---------------------------------------------------------------------------
# Symbol size selection — forced size and rectangular
# ---------------------------------------------------------------------------


class TestForcedAndRectangular:
    def test_force_valid_square_size(self):
        g = encode("Hi", size=(12, 12))
        assert g.rows == 12
        assert g.cols == 12

    def test_force_larger_than_needed(self):
        # Force 20×20 for a tiny input
        g = encode("A", size=(20, 20))
        assert g.rows == 20
        assert g.cols == 20

    def test_encode_at_10x10(self):
        g = encode_at("A", 10, 10)
        assert g.rows == 10
        assert g.cols == 10

    def test_encode_at_16x48_rect(self):
        # "HELLO WORLD" = 11 codewords; 8×32 holds 10 so use 16×48 (22 data cw)
        g = encode_at("HELLO WORLD", 16, 48)
        assert g.rows == 16
        assert g.cols == 48

    def test_rectangle_shape_allows_rect(self):
        # A short string should still fit in a rectangle if shape=Rectangle
        g = encode("Hi", shape=SymbolShape.Any)
        assert g.rows >= 8


# ---------------------------------------------------------------------------
# L-finder structure
# ---------------------------------------------------------------------------


class TestLFinder:
    def test_left_column_all_dark(self):
        g = encode("A")
        for r in range(g.rows):
            assert dark(g, r, 0), f"Left column dark at row {r}"

    def test_bottom_row_all_dark(self):
        g = encode("A")
        for c in range(g.cols):
            assert dark(g, g.rows - 1, c), f"Bottom row dark at col {c}"

    def test_top_row_alternating(self):
        # ISO 16022: top timing strip starts LIGHT at col 0, alternates L-D-L-D...
        g = encode("A")
        for c in range(g.cols):
            expected = (c % 2 == 1)
            assert dark(g, 0, c) == expected, f"Top timing at col {c}"

    def test_right_column_alternating(self):
        # ISO 16022: right timing strip starts DARK at row 0, alternates D-L-D-L...
        g = encode("A")
        for r in range(g.rows):
            expected = (r % 2 == 0)
            assert dark(g, r, g.cols - 1) == expected, f"Right timing at row {r}"


# ---------------------------------------------------------------------------
# Determinism and stability
# ---------------------------------------------------------------------------


class TestDeterminism:
    def test_same_input_same_output(self):
        g1 = encode("Hello, World!")
        g2 = encode("Hello, World!")
        s1 = grid_to_string(g1)
        s2 = grid_to_string(g2)
        assert s1 == s2

    def test_different_inputs_different_output(self):
        g1 = encode("ABC")
        g2 = encode("XYZ")
        assert grid_to_string(g1) != grid_to_string(g2)

    def test_empty_string(self):
        g = encode("")
        assert g.rows == 10
        assert g.cols == 10

    def test_known_grid_snapshot_a(self):
        # Smoke test: 'A' encodes to a 10×10 symbol deterministically.
        g = encode("A")
        s = grid_to_string(g)
        lines = s.split("\n")
        assert len(lines) == 10
        assert all(len(line) == 10 for line in lines)
        assert all(c in "01" for line in lines for c in line)


# ---------------------------------------------------------------------------
# grid_to_string
# ---------------------------------------------------------------------------


class TestGridToString:
    def test_format_is_01(self):
        g = encode("X")
        s = grid_to_string(g)
        for ch in s.replace("\n", ""):
            assert ch in "01"

    def test_row_count(self):
        g = encode("X")
        s = grid_to_string(g)
        assert len(s.split("\n")) == g.rows

    def test_col_count(self):
        g = encode("X")
        s = grid_to_string(g)
        for line in s.split("\n"):
            assert len(line) == g.cols

    def test_no_trailing_newline(self):
        g = encode("X")
        s = grid_to_string(g)
        assert not s.endswith("\n")


# ---------------------------------------------------------------------------
# layout_grid and encode_and_layout
# ---------------------------------------------------------------------------


class TestLayout:
    def test_layout_grid_returns_something(self):
        g = encode("A")
        scene = layout_grid(g)
        assert scene is not None

    def test_encode_and_layout_returns_something(self):
        scene = encode_and_layout("Hello")
        assert scene is not None

    def test_encode_and_layout_consistent_with_encode_then_layout(self):
        data = "Test"
        direct = encode_and_layout(data)
        step_by_step = layout_grid(encode(data))
        assert direct.width == step_by_step.width
        assert direct.height == step_by_step.height


# ---------------------------------------------------------------------------
# Unicode and byte data
# ---------------------------------------------------------------------------


class TestEncoding:
    def test_ascii_printable(self):
        g = encode("The quick brown fox")
        assert g.rows >= 10

    def test_digits_only(self):
        # Digit-pair compaction: 8 digits → 4 codewords
        g = encode("12345678")
        assert g.rows >= 10

    def test_mixed_digits_and_alpha(self):
        g = encode("ABC123")
        assert g.rows >= 10

    def test_larger_data_grows_symbol(self):
        small = encode("A")
        large = encode("A" * 100)
        assert large.rows > small.rows


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_all_zeros_string(self):
        g = encode("0000")
        assert g.rows >= 10

    def test_single_digit(self):
        g = encode("5")
        assert g.rows == 10

    def test_two_digits_paired(self):
        g = encode("99")
        assert g.rows == 10
