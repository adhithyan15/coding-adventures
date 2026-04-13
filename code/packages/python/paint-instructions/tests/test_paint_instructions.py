"""Tests for paint_instructions."""

from paint_instructions import PaintScene, paint_rect, paint_scene


def test_rect_builder() -> None:
    rect = paint_rect(1, 2, 3, 4, metadata={"role": "bar"})
    assert rect.kind == "rect"
    assert rect.metadata["role"] == "bar"


def test_scene_builder() -> None:
    scene = paint_scene(10, 20, [paint_rect(0, 0, 5, 6)])
    assert isinstance(scene, PaintScene)
    assert scene.width == 10
    assert scene.height == 20
