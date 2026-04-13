import barcode_1d


def test_build_scene() -> None:
    scene = barcode_1d.build_scene("HELLO-123", symbology="code39")
    assert scene.width > 0
    assert scene.height == barcode_1d.DEFAULT_RENDER_CONFIG.bar_height
    assert scene.background == "#ffffff"


def test_current_backend_probe() -> None:
    assert barcode_1d.current_backend() in {"metal", None}


def test_render_png_when_backend_is_available() -> None:
    if barcode_1d.current_backend() != "metal":
        return

    png = barcode_1d.render_png("HELLO-123", symbology="code39")
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    assert len(png) > 100
