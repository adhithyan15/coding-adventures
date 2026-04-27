from __future__ import annotations

from paint_instructions import paint_rect, paint_scene
from paint_vm_ascii import AsciiOptions, __version__, render


def test_version() -> None:
    assert __version__ == "0.1.0"


def test_render_filled_rect() -> None:
    scene = paint_scene(3, 2, [paint_rect(0, 0, 2, 1, "#000000")])
    result = render(scene, AsciiOptions(scale_x=1, scale_y=1))
    assert "\u2588" in result


def test_transparent_rect_is_empty() -> None:
    scene = paint_scene(3, 2, [paint_rect(0, 0, 2, 1, "transparent")])
    result = render(scene, AsciiOptions(scale_x=1, scale_y=1))
    assert result == ""
