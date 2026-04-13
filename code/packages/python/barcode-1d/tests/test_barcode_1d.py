import barcode_1d
import pytest


def test_build_scene() -> None:
    scene = barcode_1d.build_scene("HELLO-123", symbology="code39")
    assert scene.width > 0
    assert scene.height == barcode_1d.DEFAULT_RENDER_CONFIG.bar_height
    assert scene.background == "#ffffff"


@pytest.mark.parametrize(
    ("symbology", "data"),
    [
        ("codabar", "40156"),
        ("code128", "Code 128"),
        ("ean13", "400638133393"),
        ("itf", "123456"),
        ("upca", "03600029145"),
    ],
)
def test_build_scene_for_additional_symbologies(symbology: str, data: str) -> None:
    scene = barcode_1d.build_scene(data, symbology=symbology)
    assert scene.width > 0
    assert scene.metadata["symbology"]


def test_current_backend_probe() -> None:
    assert barcode_1d.current_backend() in {"metal", None}


def test_build_scene_rejects_unsupported_symbology() -> None:
    with pytest.raises(barcode_1d.UnsupportedSymbologyError):
        barcode_1d.build_scene("HELLO-123", symbology="qr")


def test_render_pixels_fails_without_native_backend(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(barcode_1d, "current_backend", lambda: None)

    with pytest.raises(barcode_1d.BackendUnavailableError):
        barcode_1d.render_pixels("HELLO-123", symbology="code39")


def test_render_pixels_reports_missing_native_module(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(barcode_1d, "current_backend", lambda: "metal")

    def fake_import(_module_name: str):
        raise ImportError("native extension missing")

    monkeypatch.setattr(barcode_1d.importlib, "import_module", fake_import)

    with pytest.raises(barcode_1d.BackendUnavailableError):
        barcode_1d.render_pixels("HELLO-123", symbology="code39")


def test_render_png_uses_codec_module(monkeypatch: pytest.MonkeyPatch) -> None:
    class FakeVmModule:
        @staticmethod
        def render(_scene):
            return barcode_1d.PixelContainer(2, 1, bytes([0, 0, 0, 255, 255, 255, 255, 255]))

    class FakeCodecModule:
        @staticmethod
        def encode(pixels):
            return b"PNG:" + pixels.data

    def fake_import(module_name: str):
        if module_name == "paint_vm_metal_native":
            return FakeVmModule()
        if module_name == "paint_codec_png_native":
            return FakeCodecModule()
        raise AssertionError(f"unexpected module import: {module_name}")

    monkeypatch.setattr(barcode_1d, "current_backend", lambda: "metal")
    monkeypatch.setattr(barcode_1d.importlib, "import_module", fake_import)

    png = barcode_1d.render_png("HELLO-123", symbology="code39")
    assert png.startswith(b"PNG:")


def test_render_png_when_backend_is_available() -> None:
    if barcode_1d.current_backend() != "metal":
        return

    png = barcode_1d.render_png("HELLO-123", symbology="code39")
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    assert len(png) > 100
