from paint_instructions import paint_rect, paint_scene
from paint_vm_metal_native import available, render, supported_runtime


def test_runtime_probe_matches_availability() -> None:
    if supported_runtime():
        assert available()
    else:
        assert not available()


def test_render_rect_scene() -> None:
    if not available():
        return

    scene = paint_scene(
        40,
        20,
        [paint_rect(10, 0, 20, 20, "#000000")],
        "#ffffff",
    )
    pixels = render(scene)
    assert pixels.width == 40
    assert pixels.height == 20
    assert tuple(pixels.data[0:4]) == (255, 255, 255, 255)
