"""Tests for draw-instructions."""

from draw_instructions import (
    __version__,
    create_scene,
    draw_clip,
    draw_group,
    draw_line,
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


# ---------------------------------------------------------------------------
# New primitive tests
# ---------------------------------------------------------------------------


def test_draw_line_creates_correct_instruction() -> None:
    line = draw_line(0, 0, 100, 200, stroke="#ff0000", stroke_width=2.5)
    assert line.kind == "line"
    assert line.x1 == 0
    assert line.y1 == 0
    assert line.x2 == 100
    assert line.y2 == 200
    assert line.stroke == "#ff0000"
    assert line.stroke_width == 2.5
    assert line.metadata == {}


def test_draw_line_defaults() -> None:
    line = draw_line(1, 2, 3, 4)
    assert line.stroke == "#000000"
    assert line.stroke_width == 1.0


def test_draw_clip_creates_correct_instruction() -> None:
    child = draw_rect(0, 0, 50, 50)
    clip = draw_clip(10, 10, 80, 80, [child], {"region": "main"})
    assert clip.kind == "clip"
    assert clip.x == 10
    assert clip.y == 10
    assert clip.width == 80
    assert clip.height == 80
    assert len(clip.children) == 1
    assert clip.children[0] is child
    assert clip.metadata == {"region": "main"}


def test_draw_clip_children_are_tuple() -> None:
    """Children should be stored as a tuple so the dataclass stays frozen."""
    clip = draw_clip(0, 0, 10, 10, [draw_rect(0, 0, 5, 5)])
    assert isinstance(clip.children, tuple)


def test_draw_rect_with_stroke() -> None:
    rect = draw_rect(0, 0, 10, 10, stroke="#0000ff", stroke_width=3.0)
    assert rect.stroke == "#0000ff"
    assert rect.stroke_width == 3.0


def test_draw_rect_stroke_defaults_to_none() -> None:
    rect = draw_rect(0, 0, 10, 10)
    assert rect.stroke is None
    assert rect.stroke_width is None


def test_draw_text_with_font_weight() -> None:
    text = draw_text(10, 20, "bold text", font_weight="bold")
    assert text.font_weight == "bold"


def test_draw_text_font_weight_default_is_none() -> None:
    text = draw_text(10, 20, "normal text")
    assert text.font_weight is None
