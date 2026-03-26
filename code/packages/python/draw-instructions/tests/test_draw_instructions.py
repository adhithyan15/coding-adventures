"""Tests for draw-instructions."""

from draw_instructions import (
    __version__,
    create_scene,
    draw_group,
    draw_rect,
    draw_text,
    render_with,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_draw_rect_helper() -> None:
    rect = draw_rect(1, 2, 3, 4, "#111111", {"kind": "demo"})
    assert rect.kind == "rect"
    assert rect.width == 3
    assert rect.metadata["kind"] == "demo"


def test_draw_text_helper_defaults() -> None:
    text = draw_text(10, 20, "hello")
    assert text.kind == "text"
    assert text.font_family == "monospace"
    assert text.align == "middle"


def test_group_and_scene() -> None:
    scene = create_scene(100, 50, [draw_group([draw_rect(0, 0, 5, 5)])])
    assert scene.width == 100
    assert scene.background == "#ffffff"


def test_render_with_delegates() -> None:
    scene = create_scene(10, 20, [])

    class DemoRenderer:
        def render(self, input_scene):  # noqa: ANN001
            return f"{input_scene.width}x{input_scene.height}"

    assert render_with(scene, DemoRenderer()) == "10x20"
