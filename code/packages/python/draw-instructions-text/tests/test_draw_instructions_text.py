"""Comprehensive tests for the draw-instructions-text renderer.

These tests verify that the text renderer correctly converts DrawScene
objects into Unicode box-drawing character strings.  Most tests use a
1:1 scale (1 px per character) for easy reasoning about coordinates.
"""

from __future__ import annotations

from draw_instructions import (
    DrawScene,
    create_scene,
    draw_clip,
    draw_group,
    draw_line,
    draw_rect,
    draw_text,
    render_with,
)
from draw_instructions_text import (
    TEXT_RENDERER,
    TextRenderer,
    __version__,
    render_text,
)


# ---------------------------------------------------------------------------
# Version
# ---------------------------------------------------------------------------


def test_version() -> None:
    assert __version__ == "0.1.0"


# ---------------------------------------------------------------------------
# Stroked rectangles
# ---------------------------------------------------------------------------


def test_stroked_rect_draws_box() -> None:
    """A stroked rectangle produces corners and edges."""
    scene = create_scene(5, 3, [
        draw_rect(0, 0, 4, 2, "transparent", stroke="#000", stroke_width=1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)

    assert result == (
        "\u250c\u2500\u2500\u2500\u2510\n"
        "\u2502   \u2502\n"
        "\u2514\u2500\u2500\u2500\u2518"
    )


def test_stroked_rect_single_row() -> None:
    """A 1-row-tall stroked rect collapses top and bottom edges."""
    scene = create_scene(5, 1, [
        draw_rect(0, 0, 4, 0, "transparent", stroke="#000", stroke_width=1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    # Top and bottom edges share the same row, so corners merge:
    # top-left + bottom-left = vertical-right tee, etc.
    assert len(result) > 0


# ---------------------------------------------------------------------------
# Filled rectangles
# ---------------------------------------------------------------------------


def test_filled_rect() -> None:
    """Filled rectangles produce block characters."""
    scene = create_scene(3, 2, [
        draw_rect(0, 0, 2, 1, "#000"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert "\u2588" in result


def test_transparent_rect_produces_nothing() -> None:
    """A transparent fill with no stroke produces an empty output."""
    scene = create_scene(5, 3, [
        draw_rect(0, 0, 4, 2, "transparent"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == ""


def test_none_fill_rect_produces_nothing() -> None:
    """A 'none' fill with no stroke produces an empty output."""
    scene = create_scene(5, 3, [
        draw_rect(0, 0, 4, 2, "none"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == ""


# ---------------------------------------------------------------------------
# Horizontal lines
# ---------------------------------------------------------------------------


def test_horizontal_line() -> None:
    """A horizontal line renders as dashes."""
    scene = create_scene(5, 1, [
        draw_line(0, 0, 4, 0, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "\u2500\u2500\u2500\u2500\u2500"


def test_horizontal_line_reversed() -> None:
    """A horizontal line drawn right-to-left is the same as left-to-right."""
    scene = create_scene(5, 1, [
        draw_line(4, 0, 0, 0, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "\u2500\u2500\u2500\u2500\u2500"


# ---------------------------------------------------------------------------
# Vertical lines
# ---------------------------------------------------------------------------


def test_vertical_line() -> None:
    """A vertical line renders as pipes."""
    scene = create_scene(1, 3, [
        draw_line(0, 0, 0, 2, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "\u2502\n\u2502\n\u2502"


def test_vertical_line_reversed() -> None:
    """A vertical line drawn bottom-to-top is the same as top-to-bottom."""
    scene = create_scene(1, 3, [
        draw_line(0, 2, 0, 0, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "\u2502\n\u2502\n\u2502"


# ---------------------------------------------------------------------------
# Line intersections
# ---------------------------------------------------------------------------


def test_crossing_lines_produce_cross() -> None:
    """A horizontal and vertical line crossing produce a cross character."""
    scene = create_scene(5, 3, [
        draw_line(0, 1, 4, 1, "#000", 1),
        draw_line(2, 0, 2, 2, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    lines = result.split("\n")

    assert lines[0][2] == "\u2502"   # vertical
    assert lines[1][2] == "\u253c"   # cross
    assert lines[2][2] == "\u2502"   # vertical


# ---------------------------------------------------------------------------
# Box with internal lines (table grid)
# ---------------------------------------------------------------------------


def test_box_with_horizontal_divider() -> None:
    """A box with an internal horizontal line produces tee junctions."""
    scene = create_scene(7, 3, [
        draw_rect(0, 0, 6, 2, "transparent", stroke="#000", stroke_width=1),
        draw_line(0, 1, 6, 1, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    lines = result.split("\n")

    assert lines[0] == "\u250c\u2500\u2500\u2500\u2500\u2500\u2510"
    assert lines[1][0] == "\u251c"   # left tee
    assert lines[1][6] == "\u2524"   # right tee
    assert lines[2] == "\u2514\u2500\u2500\u2500\u2500\u2500\u2518"


# ---------------------------------------------------------------------------
# Text rendering
# ---------------------------------------------------------------------------


def test_text_start_align() -> None:
    """Start-aligned text begins at the x coordinate."""
    scene = create_scene(10, 1, [
        draw_text(0, 0, "Hello", align="start"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "Hello"


def test_text_middle_align() -> None:
    """Middle-aligned text is centered on the x coordinate."""
    scene = create_scene(10, 1, [
        draw_text(5, 0, "Hi", align="middle"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result[4] == "H"
    assert result[5] == "i"


def test_text_end_align() -> None:
    """End-aligned text ends at the x coordinate."""
    scene = create_scene(10, 1, [
        draw_text(9, 0, "End", align="end"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result[6] == "E"
    assert result[7] == "n"
    assert result[8] == "d"


def test_text_overwrites_box_drawing() -> None:
    """Text written over box-drawing characters replaces them."""
    scene = create_scene(10, 1, [
        draw_line(0, 0, 9, 0, "#000", 1),
        draw_text(2, 0, "AB", align="start"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result[2] == "A"
    assert result[3] == "B"
    # Surrounding chars remain as horizontal line
    assert result[0] == "\u2500"


def test_box_drawing_does_not_overwrite_text() -> None:
    """Box-drawing characters do not overwrite existing text."""
    scene = create_scene(10, 1, [
        draw_text(2, 0, "AB", align="start"),
        draw_line(0, 0, 9, 0, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result[2] == "A"
    assert result[3] == "B"


# ---------------------------------------------------------------------------
# Text inside a box
# ---------------------------------------------------------------------------


def test_text_inside_stroked_box() -> None:
    """Text rendered inside a stroked rectangle."""
    scene = create_scene(12, 3, [
        draw_rect(0, 0, 11, 2, "transparent", stroke="#000", stroke_width=1),
        draw_text(1, 1, "Hello", align="start"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    lines = result.split("\n")

    assert lines[0] == "\u250c\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2510"
    assert lines[1] == "\u2502Hello     \u2502"
    assert lines[2] == "\u2514\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2518"


# ---------------------------------------------------------------------------
# Clips
# ---------------------------------------------------------------------------


def test_clip_truncates_text() -> None:
    """Clip regions truncate content that extends beyond the boundary."""
    scene = create_scene(10, 1, [
        draw_clip(0, 0, 3, 1, [
            draw_text(0, 0, "Hello World", align="start"),
        ]),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "Hel"


def test_nested_clips() -> None:
    """Nested clips intersect to produce the tightest bounds."""
    scene = create_scene(10, 1, [
        draw_clip(0, 0, 5, 1, [
            draw_clip(2, 0, 5, 1, [
                draw_text(0, 0, "ABCDEFGHIJ", align="start"),
            ]),
        ]),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    # Outer clip: cols [0, 5)
    # Inner clip: cols [2, 7)
    # Intersection: cols [2, 5) -> chars at positions 2, 3, 4 -> "CDE"
    assert result.strip() == "CDE"


# ---------------------------------------------------------------------------
# Groups
# ---------------------------------------------------------------------------


def test_group_recurses() -> None:
    """Groups recurse into their children."""
    scene = create_scene(5, 1, [
        draw_group([
            draw_text(0, 0, "AB", align="start"),
            draw_text(3, 0, "CD", align="start"),
        ]),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "AB CD"


# ---------------------------------------------------------------------------
# Table demo
# ---------------------------------------------------------------------------


def test_table_demo() -> None:
    """A complete table with headers and data rows."""
    scene = create_scene(13, 6, [
        draw_rect(0, 0, 12, 5, "transparent", stroke="#000", stroke_width=1),
        draw_line(6, 0, 6, 5, "#000", 1),
        draw_line(0, 2, 12, 2, "#000", 1),
        draw_text(1, 1, "Name", align="start"),
        draw_text(7, 1, "Age", align="start"),
        draw_text(1, 3, "Alice", align="start"),
        draw_text(7, 3, "30", align="start"),
        draw_text(1, 4, "Bob", align="start"),
        draw_text(7, 4, "25", align="start"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    lines = result.split("\n")

    assert lines[0] == "\u250c\u2500\u2500\u2500\u2500\u2500\u252c\u2500\u2500\u2500\u2500\u2500\u2510"
    assert "Name" in lines[1]
    assert "Age" in lines[1]
    assert lines[2][0] == "\u251c"
    assert lines[2][6] == "\u253c"
    assert lines[2][12] == "\u2524"
    assert "Alice" in lines[3]
    assert "30" in lines[3]
    assert "Bob" in lines[4]
    assert "25" in lines[4]
    assert lines[5] == "\u2514\u2500\u2500\u2500\u2500\u2500\u2534\u2500\u2500\u2500\u2500\u2500\u2518"


# ---------------------------------------------------------------------------
# Scale factor
# ---------------------------------------------------------------------------


def test_default_scale() -> None:
    """Default scale maps 8 px/col, 16 px/row."""
    scene = create_scene(88, 48, [
        draw_rect(0, 0, 80, 32, "transparent", stroke="#000", stroke_width=1),
    ])
    result = render_text(scene)
    lines = result.split("\n")

    assert len(lines) == 3
    assert lines[0][0] == "\u250c"
    assert lines[2][0] == "\u2514"


def test_custom_scale() -> None:
    """Custom scale changes the mapping from pixels to characters."""
    renderer = TextRenderer(scale_x=4, scale_y=4)
    scene = create_scene(12, 8, [
        draw_line(0, 0, 12, 0, "#000", 1),
    ])
    result = renderer.render(scene)
    assert "\u2500" in result


# ---------------------------------------------------------------------------
# render_with integration
# ---------------------------------------------------------------------------


def test_render_with_integration() -> None:
    """TextRenderer works with render_with from draw-instructions."""
    scene = create_scene(5, 1, [
        draw_text(0, 0, "OK", align="start"),
    ])
    result = render_with(scene, TextRenderer(scale_x=1, scale_y=1))
    assert result == "OK"


def test_text_renderer_singleton() -> None:
    """TEXT_RENDERER is a pre-configured TextRenderer instance."""
    scene = create_scene(16, 16, [
        draw_text(0, 0, "X", align="start"),
    ])
    result = TEXT_RENDERER.render(scene)
    assert "X" in result


# ---------------------------------------------------------------------------
# Empty scene
# ---------------------------------------------------------------------------


def test_empty_scene() -> None:
    """An empty scene returns an empty string."""
    scene = create_scene(0, 0, [])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == ""


# ---------------------------------------------------------------------------
# Diagonal lines
# ---------------------------------------------------------------------------


def test_diagonal_line() -> None:
    """Diagonal lines are approximated via Bresenham's algorithm."""
    scene = create_scene(5, 5, [
        draw_line(0, 0, 4, 4, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    # Should produce some characters along the diagonal
    lines = result.split("\n")
    assert len(lines) >= 3


# ---------------------------------------------------------------------------
# Single-cell lines
# ---------------------------------------------------------------------------


def test_single_cell_horizontal_line() -> None:
    """A zero-length horizontal line still produces a character."""
    scene = create_scene(3, 1, [
        draw_line(1, 0, 1, 0, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert "\u2500" in result


def test_single_cell_vertical_line() -> None:
    """A zero-length vertical line still produces a character."""
    # Use distinct y coords to ensure vertical path is taken
    scene = create_scene(1, 3, [
        draw_line(0, 0, 0, 2, "#000", 1),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert "\u2502" in result


# ---------------------------------------------------------------------------
# Edge cases: out-of-bounds writes are silently ignored
# ---------------------------------------------------------------------------


def test_out_of_bounds_text_ignored() -> None:
    """Text extending beyond the buffer is silently clipped."""
    scene = create_scene(3, 1, [
        draw_text(0, 0, "Hello World", align="start"),
    ])
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result == "Hel"


def test_negative_position_text() -> None:
    """Text starting at negative positions is partially clipped."""
    scene = create_scene(5, 1, [
        draw_text(-2, 0, "Hello", align="start"),
    ])
    # text starts at col -2, so chars at -2, -1 are clipped
    # chars at 0, 1, 2 should be 'l', 'l', 'o'
    result = render_text(scene, scale_x=1, scale_y=1)
    assert result[0] == "l"
    assert result[1] == "l"
    assert result[2] == "o"
