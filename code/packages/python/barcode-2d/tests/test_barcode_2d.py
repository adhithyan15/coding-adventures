"""Tests for the barcode-2d Python package.

This test suite verifies the correctness and robustness of:

- ``make_module_grid()`` — grid creation
- ``set_module()`` — immutable module updates
- ``layout()`` — ModuleGrid → PaintScene conversion (square and hex)
- Error types — invalid configuration detection

Coverage target: ≥ 90%.
"""

from __future__ import annotations

import math

import pytest

from barcode_2d import (
    Barcode2DLayoutConfig,
    AnnotatedModuleGrid,
    InvalidBarcode2DConfigError,
    ModuleAnnotation,
    ModuleGrid,
    PaintScene,
    make_module_grid,
    set_module,
    layout,
    DEFAULT_BARCODE_2D_LAYOUT_CONFIG,
)
from paint_instructions import PaintRectInstruction, PaintPathInstruction


# ============================================================================
# make_module_grid
# ============================================================================


class TestMakeModuleGrid:
    """Tests for ``make_module_grid()``."""

    def test_creates_correct_dimensions(self) -> None:
        """Grid reports the requested rows and cols."""
        grid = make_module_grid(5, 7)
        assert grid.rows == 5
        assert grid.cols == 7

    def test_all_modules_start_false(self) -> None:
        """Every module in a fresh grid is False (light)."""
        grid = make_module_grid(4, 4)
        for row in range(4):
            for col in range(4):
                assert grid.modules[row][col] is False

    def test_modules_length_matches_dimensions(self) -> None:
        """The modules tuple has the expected shape."""
        grid = make_module_grid(3, 5)
        assert len(grid.modules) == 3
        for row in grid.modules:
            assert len(row) == 5

    def test_default_shape_is_square(self) -> None:
        """Default module_shape is 'square'."""
        grid = make_module_grid(10, 10)
        assert grid.module_shape == "square"

    def test_explicit_hex_shape(self) -> None:
        """Explicit 'hex' shape is stored correctly."""
        grid = make_module_grid(33, 30, module_shape="hex")
        assert grid.module_shape == "hex"

    def test_returns_module_grid_instance(self) -> None:
        """Return type is ModuleGrid."""
        grid = make_module_grid(1, 1)
        assert isinstance(grid, ModuleGrid)

    def test_single_cell_grid(self) -> None:
        """A 1×1 grid works correctly."""
        grid = make_module_grid(1, 1)
        assert grid.rows == 1
        assert grid.cols == 1
        assert grid.modules[0][0] is False

    def test_asymmetric_grid(self) -> None:
        """Rectangular (non-square) grids are supported."""
        grid = make_module_grid(2, 10)
        assert grid.rows == 2
        assert grid.cols == 10


# ============================================================================
# set_module
# ============================================================================


class TestSetModule:
    """Tests for ``set_module()``."""

    def test_returns_new_grid(self) -> None:
        """set_module returns a new object; the original is unchanged."""
        g = make_module_grid(3, 3)
        g2 = set_module(g, 1, 1, True)
        assert g is not g2

    def test_set_dark_module(self) -> None:
        """A module can be set to dark (True)."""
        g = make_module_grid(3, 3)
        g2 = set_module(g, 0, 0, True)
        assert g2.modules[0][0] is True

    def test_original_unchanged_after_set(self) -> None:
        """The original grid is not mutated by set_module."""
        g = make_module_grid(3, 3)
        set_module(g, 1, 1, True)
        assert g.modules[1][1] is False

    def test_set_module_to_false(self) -> None:
        """A module can be set back to light (False)."""
        g = make_module_grid(3, 3)
        g2 = set_module(g, 2, 2, True)
        g3 = set_module(g2, 2, 2, False)
        assert g3.modules[2][2] is False

    def test_other_modules_unchanged(self) -> None:
        """Only the target module changes; all others stay the same."""
        g = make_module_grid(3, 3)
        g2 = set_module(g, 1, 1, True)
        for row in range(3):
            for col in range(3):
                if (row, col) != (1, 1):
                    assert g2.modules[row][col] is False

    def test_immutability_shared_rows(self) -> None:
        """Rows not affected by set_module are shared between grids."""
        g = make_module_grid(5, 5)
        g2 = set_module(g, 2, 2, True)
        # Rows 0, 1, 3, 4 should be the same tuple objects.
        for row in [0, 1, 3, 4]:
            assert g.modules[row] is g2.modules[row]

    def test_raises_index_error_for_negative_row(self) -> None:
        """Negative row index raises IndexError."""
        g = make_module_grid(3, 3)
        with pytest.raises(IndexError, match="row"):
            set_module(g, -1, 0, True)

    def test_raises_index_error_for_row_out_of_bounds(self) -> None:
        """Row index equal to grid.rows raises IndexError."""
        g = make_module_grid(3, 3)
        with pytest.raises(IndexError, match="row"):
            set_module(g, 3, 0, True)

    def test_raises_index_error_for_negative_col(self) -> None:
        """Negative col index raises IndexError."""
        g = make_module_grid(3, 3)
        with pytest.raises(IndexError, match="col"):
            set_module(g, 0, -1, True)

    def test_raises_index_error_for_col_out_of_bounds(self) -> None:
        """Col index equal to grid.cols raises IndexError."""
        g = make_module_grid(3, 3)
        with pytest.raises(IndexError, match="col"):
            set_module(g, 0, 3, True)

    def test_set_module_preserves_module_shape(self) -> None:
        """module_shape is preserved after set_module."""
        g = make_module_grid(3, 3, module_shape="hex")
        g2 = set_module(g, 0, 0, True)
        assert g2.module_shape == "hex"

    def test_chained_set_module(self) -> None:
        """Multiple set_module calls can be chained immutably."""
        g = make_module_grid(3, 3)
        g = set_module(g, 0, 0, True)
        g = set_module(g, 1, 1, True)
        g = set_module(g, 2, 2, True)
        assert g.modules[0][0] is True
        assert g.modules[1][1] is True
        assert g.modules[2][2] is True
        assert g.modules[0][1] is False


# ============================================================================
# layout — square modules
# ============================================================================


class TestLayoutSquare:
    """Tests for ``layout()`` with square module grids."""

    def _make_all_dark_grid(self, rows: int, cols: int) -> ModuleGrid:
        """Helper: create a grid with every module dark."""
        g = make_module_grid(rows, cols)
        for r in range(rows):
            for c in range(cols):
                g = set_module(g, r, c, True)
        return g

    def test_returns_paint_scene(self) -> None:
        """layout() returns a PaintScene instance."""
        grid = make_module_grid(3, 3)
        scene = layout(grid)
        assert isinstance(scene, PaintScene)

    def test_total_width_with_default_config(self) -> None:
        """Total width = (cols + 2 * quiet_zone) * module_size_px."""
        grid = make_module_grid(5, 5)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=4)
        scene = layout(grid, cfg)
        expected_width = (5 + 2 * 4) * 10  # 130
        assert scene.width == expected_width

    def test_total_height_with_default_config(self) -> None:
        """Total height = (rows + 2 * quiet_zone) * module_size_px."""
        grid = make_module_grid(5, 5)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=4)
        scene = layout(grid, cfg)
        expected_height = (5 + 2 * 4) * 10  # 130
        assert scene.height == expected_height

    def test_asymmetric_grid_dimensions(self) -> None:
        """Non-square grids produce the correct width and height."""
        grid = make_module_grid(rows=3, cols=7)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.width == 70   # 7 cols * 10 px
        assert scene.height == 30  # 3 rows * 10 px

    def test_background_is_first_instruction(self) -> None:
        """First instruction is always the background rect."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(
            module_size_px=10,
            quiet_zone_modules=1,
            background="#ffffff",
        )
        scene = layout(grid, cfg)
        first = scene.instructions[0]
        assert isinstance(first, PaintRectInstruction)
        assert first.x == 0
        assert first.y == 0
        assert first.fill == "#ffffff"

    def test_background_rect_covers_entire_scene(self) -> None:
        """Background rect is exactly scene width × scene height."""
        grid = make_module_grid(5, 5)
        cfg = Barcode2DLayoutConfig(module_size_px=8, quiet_zone_modules=2)
        scene = layout(grid, cfg)
        bg = scene.instructions[0]
        assert isinstance(bg, PaintRectInstruction)
        assert bg.width == scene.width
        assert bg.height == scene.height

    def test_no_dark_module_rects_for_all_light_grid(self) -> None:
        """An all-light grid produces exactly 1 instruction (background)."""
        grid = make_module_grid(3, 3)
        scene = layout(grid)
        assert len(scene.instructions) == 1

    def test_single_dark_module_produces_two_instructions(self) -> None:
        """One dark module → background rect + one dark rect."""
        grid = make_module_grid(3, 3)
        grid = set_module(grid, 1, 1, True)
        scene = layout(grid)
        assert len(scene.instructions) == 2

    def test_dark_module_rect_pixel_position(self) -> None:
        """Dark module at (row, col) is placed at the correct pixel offset."""
        grid = make_module_grid(5, 5)
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        dark_rect = scene.instructions[1]
        assert isinstance(dark_rect, PaintRectInstruction)
        assert dark_rect.x == 0
        assert dark_rect.y == 0
        assert dark_rect.width == 10
        assert dark_rect.height == 10

    def test_dark_module_offset_by_quiet_zone(self) -> None:
        """Dark module position is offset by the quiet zone in pixels."""
        grid = make_module_grid(3, 3)
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=2)
        scene = layout(grid, cfg)
        dark_rect = scene.instructions[1]
        assert isinstance(dark_rect, PaintRectInstruction)
        # quiet_zone_px = 2 * 10 = 20
        assert dark_rect.x == 20
        assert dark_rect.y == 20

    def test_dark_module_in_middle_of_grid(self) -> None:
        """Dark module in the middle of the grid has correct pixel coords."""
        grid = make_module_grid(5, 5)
        grid = set_module(grid, 2, 3, True)
        cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        dark_rect = scene.instructions[1]
        assert isinstance(dark_rect, PaintRectInstruction)
        assert dark_rect.x == 30   # col 3 * 10 px
        assert dark_rect.y == 20   # row 2 * 10 px

    def test_dark_module_fill_uses_foreground(self) -> None:
        """Dark module rect uses the configured foreground colour."""
        grid = make_module_grid(3, 3)
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(foreground="#112233", module_size_px=10, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.instructions[1].fill == "#112233"

    def test_all_dark_grid_instruction_count(self) -> None:
        """All-dark 3×3 grid → 1 background + 9 dark rects = 10 instructions."""
        grid = self._make_all_dark_grid(3, 3)
        scene = layout(grid)
        # 1 background + 9 dark rects
        assert len(scene.instructions) == 10

    def test_scene_background_colour(self) -> None:
        """PaintScene.background matches the configured background colour."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(background="#aabbcc", module_size_px=10, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.background == "#aabbcc"

    def test_none_config_uses_defaults(self) -> None:
        """Passing no config (None) uses DEFAULT_BARCODE_2D_LAYOUT_CONFIG."""
        grid = make_module_grid(21, 21)
        scene = layout(grid)
        expected_width = (21 + 2 * DEFAULT_BARCODE_2D_LAYOUT_CONFIG.quiet_zone_modules) * DEFAULT_BARCODE_2D_LAYOUT_CONFIG.module_size_px
        assert scene.width == expected_width

    def test_zero_quiet_zone(self) -> None:
        """A quiet_zone_modules of 0 means no padding around the grid."""
        grid = make_module_grid(5, 5)
        cfg = Barcode2DLayoutConfig(module_size_px=4, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.width == 20
        assert scene.height == 20


# ============================================================================
# layout — hex modules
# ============================================================================


class TestLayoutHex:
    """Tests for ``layout()`` with hex module grids (MaxiCode style)."""

    def test_returns_paint_scene(self) -> None:
        """layout() with a hex grid returns a PaintScene."""
        grid = make_module_grid(3, 3, module_shape="hex")
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert isinstance(scene, PaintScene)

    def test_background_is_first_instruction(self) -> None:
        """First instruction is the background rect even for hex grids."""
        grid = make_module_grid(3, 3, module_shape="hex")
        cfg = Barcode2DLayoutConfig(
            module_shape="hex",
            background="#eeeeee",
            quiet_zone_modules=0,
        )
        scene = layout(grid, cfg)
        bg = scene.instructions[0]
        assert isinstance(bg, PaintRectInstruction)
        assert bg.fill == "#eeeeee"

    def test_dark_hex_module_produces_path_instruction(self) -> None:
        """A dark hex module becomes a PaintPathInstruction."""
        grid = make_module_grid(3, 3, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        dark_instr = scene.instructions[1]
        assert isinstance(dark_instr, PaintPathInstruction)

    def test_dark_hex_module_path_has_seven_commands(self) -> None:
        """Each hexagon path has 7 commands: move_to, 5×line_to, close."""
        grid = make_module_grid(3, 3, module_shape="hex")
        grid = set_module(grid, 1, 1, True)
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        path_instr = scene.instructions[1]
        assert isinstance(path_instr, PaintPathInstruction)
        assert len(path_instr.commands) == 7

    def test_hex_path_starts_with_move_to(self) -> None:
        """First path command is 'move_to'."""
        grid = make_module_grid(2, 2, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        path_instr = scene.instructions[1]
        assert isinstance(path_instr, PaintPathInstruction)
        assert path_instr.commands[0].kind == "move_to"

    def test_hex_path_ends_with_close(self) -> None:
        """Last path command is 'close'."""
        grid = make_module_grid(2, 2, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        path_instr = scene.instructions[1]
        assert isinstance(path_instr, PaintPathInstruction)
        assert path_instr.commands[-1].kind == "close"

    def test_hex_path_middle_commands_are_line_to(self) -> None:
        """Commands 1–5 are all 'line_to'."""
        grid = make_module_grid(2, 2, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        path_instr = scene.instructions[1]
        assert isinstance(path_instr, PaintPathInstruction)
        for i in range(1, 6):
            assert path_instr.commands[i].kind == "line_to"

    def test_hex_all_light_grid_one_instruction(self) -> None:
        """All-light hex grid produces exactly 1 instruction (background)."""
        grid = make_module_grid(3, 3, module_shape="hex")
        cfg = Barcode2DLayoutConfig(module_shape="hex", quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert len(scene.instructions) == 1

    def test_hex_dark_module_fill_uses_foreground(self) -> None:
        """Dark hex module path uses the configured foreground colour."""
        grid = make_module_grid(2, 2, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        cfg = Barcode2DLayoutConfig(
            module_shape="hex",
            foreground="#ff0000",
            quiet_zone_modules=0,
        )
        scene = layout(grid, cfg)
        assert scene.instructions[1].fill == "#ff0000"

    def test_hex_total_dimensions(self) -> None:
        """Hex grid total dimensions use the expected hex geometry."""
        grid = make_module_grid(rows=4, cols=5, module_shape="hex")
        cfg = Barcode2DLayoutConfig(
            module_size_px=10,
            quiet_zone_modules=0,
            module_shape="hex",
        )
        scene = layout(grid, cfg)
        hex_width = 10.0
        hex_height = 10.0 * (math.sqrt(3) / 2)
        expected_width = int(5 * hex_width + hex_width / 2)
        expected_height = int(4 * hex_height)
        assert scene.width == expected_width
        assert scene.height == expected_height

    def test_odd_row_hex_is_offset(self) -> None:
        """A dark module on an odd row has its centre shifted by hex_width/2."""
        # We place one dark module on row 0 and one on row 1 at the same col.
        # Their x-coordinates should differ by half a hex_width.
        grid = make_module_grid(2, 1, module_shape="hex")
        grid = set_module(grid, 0, 0, True)
        grid = set_module(grid, 1, 0, True)
        cfg = Barcode2DLayoutConfig(
            module_size_px=10,
            quiet_zone_modules=0,
            module_shape="hex",
        )
        scene = layout(grid, cfg)
        # instructions[0] = background, [1] = row-0 hex, [2] = row-1 hex
        path_row0 = scene.instructions[1]
        path_row1 = scene.instructions[2]
        assert isinstance(path_row0, PaintPathInstruction)
        assert isinstance(path_row1, PaintPathInstruction)
        # The first vertex of each path gives the centre offset:
        # cx = col * hex_width + (row % 2) * (hex_width / 2) + circum_r
        # For row 0: cx = 0 + 0 + circum_r
        # For row 1: cx = 0 + 5 + circum_r  → 5 px further right
        x_row0 = path_row0.commands[0].x
        x_row1 = path_row1.commands[0].x
        assert abs(x_row1 - x_row0 - 5.0) < 1e-9


# ============================================================================
# layout — invalid configuration
# ============================================================================


class TestLayoutValidation:
    """Tests for ``layout()`` configuration validation."""

    def test_raises_on_zero_module_size_px(self) -> None:
        """module_size_px of 0 raises InvalidBarcode2DConfigError."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(module_size_px=0)
        with pytest.raises(InvalidBarcode2DConfigError, match="module_size_px"):
            layout(grid, cfg)

    def test_raises_on_negative_module_size_px(self) -> None:
        """Negative module_size_px raises InvalidBarcode2DConfigError."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(module_size_px=-5)
        with pytest.raises(InvalidBarcode2DConfigError, match="module_size_px"):
            layout(grid, cfg)

    def test_raises_on_negative_quiet_zone(self) -> None:
        """Negative quiet_zone_modules raises InvalidBarcode2DConfigError."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(quiet_zone_modules=-1)
        with pytest.raises(InvalidBarcode2DConfigError, match="quiet_zone_modules"):
            layout(grid, cfg)

    def test_raises_on_shape_mismatch_square_config_hex_grid(self) -> None:
        """Square config + hex grid raises InvalidBarcode2DConfigError."""
        grid = make_module_grid(3, 3, module_shape="hex")
        cfg = Barcode2DLayoutConfig(module_shape="square")
        with pytest.raises(InvalidBarcode2DConfigError, match="module_shape"):
            layout(grid, cfg)

    def test_raises_on_shape_mismatch_hex_config_square_grid(self) -> None:
        """Hex config + square grid raises InvalidBarcode2DConfigError."""
        grid = make_module_grid(3, 3, module_shape="square")
        cfg = Barcode2DLayoutConfig(module_shape="hex")
        with pytest.raises(InvalidBarcode2DConfigError, match="module_shape"):
            layout(grid, cfg)

    def test_zero_quiet_zone_is_valid(self) -> None:
        """quiet_zone_modules of exactly 0 is valid and does not raise."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.width == 30  # 3 * 10 px

    def test_module_size_one_is_valid(self) -> None:
        """module_size_px of 1 is the minimum valid value."""
        grid = make_module_grid(3, 3)
        cfg = Barcode2DLayoutConfig(module_size_px=1, quiet_zone_modules=0)
        scene = layout(grid, cfg)
        assert scene.width == 3
        assert scene.height == 3


# ============================================================================
# AnnotatedModuleGrid — dataclass construction
# ============================================================================


class TestAnnotatedModuleGrid:
    """Smoke tests for AnnotatedModuleGrid construction."""

    def test_constructs_annotated_grid(self) -> None:
        """AnnotatedModuleGrid can be created with annotations."""
        grid = make_module_grid(2, 2)
        ann = ModuleAnnotation(role="finder", dark=True)
        annotations: tuple[tuple[ModuleAnnotation | None, ...], ...] = (
            (ann, None),
            (None, ann),
        )
        ag = AnnotatedModuleGrid(
            cols=2,
            rows=2,
            modules=grid.modules,
            annotations=annotations,
        )
        assert ag.annotations[0][0] is ann
        assert ag.annotations[0][1] is None

    def test_module_annotation_fields(self) -> None:
        """ModuleAnnotation stores all fields correctly."""
        ann = ModuleAnnotation(
            role="data",
            dark=True,
            codeword_index=5,
            bit_index=3,
            metadata={"format_role": "qr:masked"},
        )
        assert ann.role == "data"
        assert ann.dark is True
        assert ann.codeword_index == 5
        assert ann.bit_index == 3
        assert ann.metadata["format_role"] == "qr:masked"

    def test_module_annotation_defaults(self) -> None:
        """ModuleAnnotation optional fields default to None / empty dict."""
        ann = ModuleAnnotation(role="timing", dark=False)
        assert ann.codeword_index is None
        assert ann.bit_index is None
        assert ann.metadata == {}


# ============================================================================
# DEFAULT_BARCODE_2D_LAYOUT_CONFIG
# ============================================================================


class TestDefaultConfig:
    """Tests for DEFAULT_BARCODE_2D_LAYOUT_CONFIG values."""

    def test_default_module_size(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.module_size_px == 10

    def test_default_quiet_zone(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.quiet_zone_modules == 4

    def test_default_foreground(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.foreground == "#000000"

    def test_default_background(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.background == "#ffffff"

    def test_default_module_shape(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.module_shape == "square"

    def test_default_show_annotations(self) -> None:
        assert DEFAULT_BARCODE_2D_LAYOUT_CONFIG.show_annotations is False
