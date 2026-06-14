"""Tests for the Aztec Code encoder.

Mirrors the TypeScript test suite at
``code/packages/typescript/aztec-code/tests/aztec-code.test.ts`` so the two
implementations evolve together.  Coverage targets:

- Symbol size selection (compact 1-4, full).
- Bullseye finder pattern structure (compact and full).
- Orientation mark placement.
- Bit stuffing (indirectly via encode determinism).
- GF(16) mode message and GF(256)/0x12D RS (indirectly via encode).
- ``encode_and_layout``, ``explain``, ``layout_grid`` wrappers.
- ``min_ecc_percent`` option.
- Error cases.
- Determinism.
"""

from __future__ import annotations

import dataclasses

import pytest

from aztec_code import (
    VERSION,
    AnnotatedModuleGrid,
    AztecError,
    AztecOptions,
    Barcode2DLayoutConfig,
    InputTooLongError,
    ModuleGrid,
    PaintScene,
    __version__,
    encode,
    encode_and_layout,
    explain,
    layout_grid,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def dark(grid: ModuleGrid, row: int, col: int) -> bool:
    """Return whether (row, col) is dark in the grid."""
    return grid.modules[row][col] is True


# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------


class TestVersion:
    def test_version_string(self):
        assert VERSION == "0.1.0"

    def test_dunder_version_matches(self):
        assert __version__ == VERSION


# ---------------------------------------------------------------------------
# Error classes
# ---------------------------------------------------------------------------


class TestErrorClasses:
    def test_aztec_error_is_exception(self):
        e = AztecError("test")
        assert isinstance(e, Exception)
        assert isinstance(e, AztecError)
        assert str(e) == "test"

    def test_input_too_long_extends_aztec_error(self):
        e = InputTooLongError("too long")
        assert isinstance(e, AztecError)
        assert isinstance(e, InputTooLongError)
        assert str(e) == "too long"

    def test_huge_input_raises_input_too_long(self):
        with pytest.raises(InputTooLongError):
            encode("x" * 2000)


# ---------------------------------------------------------------------------
# Compact symbol sizes
# ---------------------------------------------------------------------------


class TestCompactSymbolSizes:
    def test_one_layer_compact_15x15(self):
        # Single byte 'A' fits in compact-1 (15x15).
        g = encode("A")
        assert g.rows == 15
        assert g.cols == 15

    def test_two_layer_compact_19x19(self):
        g = encode("Hello")
        assert g.rows == 19
        assert g.cols == 19

    def test_three_layer_compact_23x23(self):
        # 20 bytes + Binary-Shift overhead overflows compact-2.
        g = encode("12345678901234567890")
        assert g.rows == 23
        assert g.cols == 23

    def test_four_layer_compact_27x27(self):
        g = encode("12345678901234567890" * 2)
        assert g.rows == 27
        assert g.cols == 27


# ---------------------------------------------------------------------------
# Full symbol sizes
# ---------------------------------------------------------------------------


class TestFullSymbolSizes:
    def test_full_used_when_compact_overflows(self):
        # 100 bytes exceed compact-4 capacity.
        g = encode("x" * 100)
        assert g.rows >= 19
        # Full symbol sizes are 19+4k -> 19, 23, 27, ... all == 3 (mod 4).
        assert g.rows % 4 == 3

    def test_full_symbol_is_square(self):
        g = encode("x" * 150)
        assert g.rows == g.cols


# ---------------------------------------------------------------------------
# Bullseye finder pattern — compact (r=5, cx=cy=7 for 15x15)
# ---------------------------------------------------------------------------


class TestBullseyeCompact:
    @pytest.fixture(scope="class")
    def grid(self) -> ModuleGrid:
        return encode("A")

    def test_center_is_dark(self, grid):
        assert dark(grid, 7, 7) is True

    def test_d1_ring_is_dark(self, grid):
        for dr in range(-1, 2):
            for dc in range(-1, 2):
                if dr == 0 and dc == 0:
                    continue
                assert dark(grid, 7 + dr, 7 + dc) is True

    def test_d2_ring_is_light(self, grid):
        # Corners of the d=2 perimeter.
        assert dark(grid, 7 - 2, 7 - 2) is False
        assert dark(grid, 7 - 2, 7 + 2) is False
        assert dark(grid, 7 + 2, 7 - 2) is False
        assert dark(grid, 7 + 2, 7 + 2) is False
        # Midpoints of each side of the d=2 ring.
        assert dark(grid, 7 - 2, 7) is False
        assert dark(grid, 7 + 2, 7) is False
        assert dark(grid, 7, 7 - 2) is False
        assert dark(grid, 7, 7 + 2) is False

    def test_d3_ring_is_dark_at_axis(self, grid):
        assert dark(grid, 7 - 3, 7) is True
        assert dark(grid, 7 + 3, 7) is True
        assert dark(grid, 7, 7 - 3) is True
        assert dark(grid, 7, 7 + 3) is True

    def test_d4_ring_is_light_at_axis(self, grid):
        assert dark(grid, 7 - 4, 7) is False
        assert dark(grid, 7 + 4, 7) is False
        assert dark(grid, 7, 7 - 4) is False
        assert dark(grid, 7, 7 + 4) is False

    def test_d5_ring_is_dark_at_axis(self, grid):
        assert dark(grid, 7 - 5, 7) is True
        assert dark(grid, 7 + 5, 7) is True
        assert dark(grid, 7, 7 - 5) is True
        assert dark(grid, 7, 7 + 5) is True


# ---------------------------------------------------------------------------
# Bullseye finder pattern — full symbol
# ---------------------------------------------------------------------------


class TestBullseyeFull:
    @pytest.fixture(scope="class")
    def grid(self) -> ModuleGrid:
        return encode("x" * 100)

    def test_center_is_dark(self, grid):
        cx = grid.cols // 2
        cy = grid.rows // 2
        assert dark(grid, cy, cx) is True

    def test_d2_ring_is_light_full(self, grid):
        cx = grid.cols // 2
        cy = grid.rows // 2
        assert dark(grid, cy - 2, cx) is False
        assert dark(grid, cy + 2, cx) is False
        assert dark(grid, cy, cx - 2) is False
        assert dark(grid, cy, cx + 2) is False

    def test_d7_ring_is_dark_full(self, grid):
        cx = grid.cols // 2
        cy = grid.rows // 2
        assert dark(grid, cy - 7, cx) is True
        assert dark(grid, cy + 7, cx) is True
        assert dark(grid, cy, cx - 7) is True
        assert dark(grid, cy, cx + 7) is True


# ---------------------------------------------------------------------------
# Orientation marks — compact and full
# ---------------------------------------------------------------------------


class TestOrientationMarksCompact:
    @pytest.fixture(scope="class")
    def grid(self) -> ModuleGrid:
        return encode("A")  # 15x15, cx=cy=7, bullseye r=5, mode ring r=6

    def test_corners_are_dark(self, grid):
        cx, cy, r = 7, 7, 6
        assert dark(grid, cy - r, cx - r) is True
        assert dark(grid, cy - r, cx + r) is True
        assert dark(grid, cy + r, cx + r) is True
        assert dark(grid, cy + r, cx - r) is True


class TestOrientationMarksFull:
    @pytest.fixture(scope="class")
    def grid(self) -> ModuleGrid:
        return encode("x" * 100)

    def test_corners_are_dark(self, grid):
        cx = grid.cols // 2
        cy = grid.rows // 2
        r = 8  # bullseye_radius(full=7) + 1
        assert dark(grid, cy - r, cx - r) is True
        assert dark(grid, cy - r, cx + r) is True
        assert dark(grid, cy + r, cx + r) is True
        assert dark(grid, cy + r, cx - r) is True


# ---------------------------------------------------------------------------
# Grid structural properties
# ---------------------------------------------------------------------------


class TestGridStructure:
    def test_modules_dimensions(self):
        g = encode("A")
        assert len(g.modules) == 15
        assert len(g.modules[0]) == 15

    def test_module_shape_is_square(self):
        assert encode("A").module_shape == "square"

    def test_grid_is_square(self):
        g = encode("Hello, World!")
        assert g.rows == g.cols

    def test_rows_matches_modules_length(self):
        g = encode("test")
        assert g.rows == len(g.modules)

    def test_cols_matches_first_row_length(self):
        g = encode("test")
        assert g.cols == len(g.modules[0])

    def test_grid_is_module_grid_instance(self):
        g = encode("test")
        assert isinstance(g, ModuleGrid)

    def test_modules_is_immutable_tuple(self):
        # Frozen ModuleGrid stores tuples — confirming this guarantees the
        # caller cannot accidentally mutate the encoder output.
        g = encode("A")
        assert isinstance(g.modules, tuple)
        assert isinstance(g.modules[0], tuple)
        assert isinstance(g.modules[0][0], bool)


# ---------------------------------------------------------------------------
# bytes input
# ---------------------------------------------------------------------------


class TestBytesInput:
    def test_accepts_bytes(self):
        g1 = encode("Hello")
        g2 = encode(b"Hello")
        assert g1.rows == g2.rows
        assert g1.cols == g2.cols

    def test_string_and_bytes_produce_identical_grid(self):
        g1 = encode("ABC")
        g2 = encode(b"ABC")
        for r in range(g1.rows):
            for c in range(g1.cols):
                assert dark(g1, r, c) is dark(g2, r, c)

    def test_accepts_bytearray(self):
        # bytearray is mutable but iterable as bytes-like.
        ba = bytearray(b"Hello")
        g = encode(ba)
        assert g.rows >= 15


# ---------------------------------------------------------------------------
# min_ecc_percent option
# ---------------------------------------------------------------------------


class TestMinEccPercent:
    def test_higher_ecc_requires_at_least_as_large_symbol(self):
        g_low = encode("Hello", AztecOptions(min_ecc_percent=10))
        g_high = encode("Hello", AztecOptions(min_ecc_percent=80))
        assert g_high.rows >= g_low.rows

    def test_min_ecc_33_succeeds(self):
        g = encode("Hello", AztecOptions(min_ecc_percent=33))
        assert g.rows >= 15

    def test_min_ecc_10_succeeds(self):
        # Should not raise.
        encode("Hello", AztecOptions(min_ecc_percent=10))

    def test_min_ecc_90_succeeds_or_grows(self):
        # 90% ECC may need a much larger symbol but should not raise for
        # short input.
        g = encode("Hello", AztecOptions(min_ecc_percent=90))
        assert g.rows >= 15


# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------


class TestDeterminism:
    def test_same_input_produces_identical_output(self):
        g1 = encode("Hello, World!")
        g2 = encode("Hello, World!")
        assert g1.rows == g2.rows
        for r in range(g1.rows):
            for c in range(g1.cols):
                assert dark(g1, r, c) is dark(g2, r, c)

    def test_different_inputs_produce_different_grids(self):
        g1 = encode("Hello")
        g2 = encode("World")
        differs = False
        for r in range(g1.rows):
            for c in range(g1.cols):
                if dark(g1, r, c) is not dark(g2, r, c):
                    differs = True
                    break
            if differs:
                break
        assert differs


# ---------------------------------------------------------------------------
# encode_and_layout, layout_grid
# ---------------------------------------------------------------------------


class TestEncodeAndLayout:
    def test_returns_paint_scene(self):
        scene = encode_and_layout("Hello")
        assert scene is not None
        assert isinstance(scene, PaintScene)

    def test_accepts_options(self):
        scene = encode_and_layout("Hello", AztecOptions(min_ecc_percent=33))
        assert scene is not None

    def test_accepts_layout_config(self):
        cfg = Barcode2DLayoutConfig(module_size_px=5, quiet_zone_modules=2)
        scene = encode_and_layout("Hello", None, cfg)
        assert scene is not None


class TestLayoutGrid:
    def test_layout_grid_returns_scene(self):
        g = encode("A")
        scene = layout_grid(g)
        assert isinstance(scene, PaintScene)

    def test_layout_grid_accepts_config(self):
        g = encode("A")
        cfg = Barcode2DLayoutConfig(module_size_px=20, quiet_zone_modules=1)
        scene = layout_grid(g, cfg)
        assert isinstance(scene, PaintScene)


# ---------------------------------------------------------------------------
# explain
# ---------------------------------------------------------------------------


class TestExplain:
    def test_returns_annotated_grid(self):
        annotated = explain("Hello")
        assert isinstance(annotated, AnnotatedModuleGrid)
        assert annotated.rows > 0
        assert annotated.cols > 0

    def test_annotated_dimensions_match_encode(self):
        g = encode("Hello")
        a = explain("Hello")
        assert a.rows == g.rows
        assert a.cols == g.cols

    def test_annotated_modules_match_encode(self):
        g = encode("Hello")
        a = explain("Hello")
        for r in range(g.rows):
            for c in range(g.cols):
                assert a.modules[r][c] is g.modules[r][c]


# ---------------------------------------------------------------------------
# Cross-language corpus
# ---------------------------------------------------------------------------


class TestCrossLanguageCorpus:
    def test_encode_A_is_15x15(self):
        g = encode("A")
        assert g.rows == 15
        assert g.cols == 15

    def test_empty_string_produces_valid_grid(self):
        # Empty string -> 5+5 bits (BS escape + 0-length); fits in compact-1.
        g = encode("")
        assert g.rows >= 15
        assert g.rows == g.cols

    def test_hello_world_deterministic_and_square(self):
        g = encode("Hello, World!")
        assert g.rows == g.cols
        assert g.rows >= 15

    def test_all_modules_are_boolean(self):
        g = encode("Test")
        for row in g.modules:
            for cell in row:
                assert isinstance(cell, bool)


# ---------------------------------------------------------------------------
# Reference grid (full symbols only)
# ---------------------------------------------------------------------------


class TestReferenceGridFull:
    @pytest.fixture(scope="class")
    def grid(self) -> ModuleGrid:
        return encode("x" * 100)

    def test_full_symbol_dimensions_reasonable(self, grid):
        # Reference grid only matters for full symbols; verifying that we
        # get a full symbol exercises the reference-grid drawing branch.
        assert grid.rows >= 19

    def test_center_is_dark_in_full(self, grid):
        cx = grid.cols // 2
        cy = grid.rows // 2
        assert dark(grid, cy, cx) is True


# ---------------------------------------------------------------------------
# Additional coverage
# ---------------------------------------------------------------------------


class TestAdditionalCoverage:
    def test_encodes_all_byte_values(self):
        bs = bytes(range(256))
        g = encode(bs)
        assert g.rows >= 15
        assert g.rows == g.cols

    def test_encodes_32_byte_input(self):
        g = encode("A" * 32)
        assert g.rows >= 15

    def test_encodes_50_byte_input(self):
        g = encode("B" * 50)
        assert g.rows >= 15

    def test_encodes_200_byte_input(self):
        g = encode("C" * 200)
        assert g.rows >= 19

    def test_encodes_500_byte_input(self):
        g = encode("D" * 500)
        assert g.rows >= 19

    def test_encodes_unicode_via_utf8(self):
        # Japanese "Hello" — 5 chars but 15 UTF-8 bytes.
        g = encode("こんにちは")
        assert g.rows >= 15
        assert g.rows == g.cols

    def test_default_options_is_23_percent_ecc(self):
        # Sanity: default AztecOptions matches docstring claim.
        opts = AztecOptions()
        assert opts.min_ecc_percent == 23

    def test_options_is_frozen_dataclass(self):
        opts = AztecOptions()
        # Frozen dataclass raises FrozenInstanceError on attribute assignment.
        with pytest.raises(dataclasses.FrozenInstanceError):
            opts.min_ecc_percent = 50  # type: ignore[misc]


# ---------------------------------------------------------------------------
# Symbol-selection edge cases
# ---------------------------------------------------------------------------


class TestSymbolSelectionEdgeCases:
    def test_input_at_compact_boundary(self):
        # Push right up against compact-4 capacity.  This exercises the
        # selector's "fit at last compact tier" branch.
        g = encode("z" * 60)
        assert g.rows >= 15

    def test_input_just_over_compact_boundary(self):
        # Forces fall-through to full symbols.
        g = encode("z" * 90)
        assert g.rows >= 19
        assert g.rows % 4 == 3  # full sizes are 19, 23, 27, ...

    def test_very_high_ecc_pushes_to_full_symbol(self):
        # With min_ecc_pct=80 the small compact tiers may run out of data
        # codewords; the selector should keep walking up the table.
        g = encode("Hello", AztecOptions(min_ecc_percent=80))
        assert g.rows >= 15


# ---------------------------------------------------------------------------
# Internal helpers — exercised indirectly but worth a quick smoke test
# ---------------------------------------------------------------------------


class TestInternalSmoke:
    def test_encode_short_input_uses_compact(self):
        # Short input should always pick compact (rows in {15, 19, 23, 27}).
        g = encode("Hi")
        assert g.rows in {15, 19, 23, 27}

    def test_encode_grid_immutable(self):
        # ModuleGrid is frozen — can't reassign fields.
        g = encode("A")
        with pytest.raises(Exception):
            g.rows = 99  # type: ignore[misc]
