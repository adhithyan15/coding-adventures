"""Tests for draw-instructions-svg."""

from draw_instructions import create_scene, draw_group, draw_rect, draw_text
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
