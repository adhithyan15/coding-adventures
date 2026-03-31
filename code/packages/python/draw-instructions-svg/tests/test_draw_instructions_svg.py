"""Tests for draw-instructions-svg."""

from draw_instructions import (
    create_scene,
    draw_clip,
    draw_group,
    draw_line,
    draw_rect,
    draw_text,
)
from draw_instructions_svg import __version__, render_svg


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_render_svg_document() -> None:
    scene = create_scene(100, 50, [draw_rect(10, 10, 20, 30)], metadata={"label": "demo"})
    svg = render_svg(scene)
    assert "<svg" in svg
    assert 'aria-label="demo"' in svg


def test_render_text_and_escape() -> None:
    scene = create_scene(100, 50, [draw_text(10, 20, "A&B")])
    assert "A&amp;B" in render_svg(scene)


def test_render_group_with_metadata() -> None:
    scene = create_scene(100, 50, [draw_group([draw_rect(0, 0, 5, 5)], {"layer": "bars"})])
    svg = render_svg(scene)
    assert "<g" in svg
    assert 'data-layer="bars"' in svg


# ---------------------------------------------------------------------------
# New primitive SVG rendering tests
# ---------------------------------------------------------------------------


def test_render_line() -> None:
    scene = create_scene(100, 50, [draw_line(0, 0, 100, 50, "#ff0000", 2.0)])
    svg = render_svg(scene)
    assert "<line" in svg
    assert 'x1="0"' in svg
    assert 'y1="0"' in svg
    assert 'x2="100"' in svg
    assert 'y2="50"' in svg
    assert 'stroke="#ff0000"' in svg
    assert 'stroke-width="2.0"' in svg


def test_render_clip() -> None:
    child = draw_rect(0, 0, 200, 200)
    scene = create_scene(100, 100, [draw_clip(10, 10, 80, 80, [child])])
    svg = render_svg(scene)
    assert "<clipPath" in svg
    assert 'id="clip-0"' in svg
    assert 'clip-path="url(#clip-0)"' in svg


def test_render_clip_counter_resets_between_calls() -> None:
    """The clip ID counter should reset each time render() is called."""
    child = draw_rect(0, 0, 10, 10)
    scene = create_scene(100, 100, [draw_clip(0, 0, 50, 50, [child])])
    svg1 = render_svg(scene)
    svg2 = render_svg(scene)
    # Both calls should produce clip-0 since the counter resets.
    assert 'id="clip-0"' in svg1
    assert 'id="clip-0"' in svg2


def test_render_rect_with_stroke() -> None:
    scene = create_scene(100, 50, [draw_rect(5, 5, 20, 20, stroke="#00ff00", stroke_width=2.0)])
    svg = render_svg(scene)
    assert 'stroke="#00ff00"' in svg
    assert 'stroke-width="2.0"' in svg


def test_render_rect_without_stroke_has_no_stroke_attr() -> None:
    scene = create_scene(100, 50, [draw_rect(5, 5, 20, 20)])
    svg = render_svg(scene)
    assert "stroke" not in svg.split("<rect")[1].split("/>")[0] or 'stroke=' not in svg.split("</svg>")[0].split("\n")[-2]


def test_render_text_bold() -> None:
    scene = create_scene(100, 50, [draw_text(10, 20, "Bold!", font_weight="bold")])
    svg = render_svg(scene)
    assert 'font-weight="bold"' in svg


def test_render_text_normal_no_weight_attr() -> None:
    scene = create_scene(100, 50, [draw_text(10, 20, "Normal")])
    svg = render_svg(scene)
    assert "font-weight" not in svg
