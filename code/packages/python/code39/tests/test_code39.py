"""Tests for code39."""

from code39 import (
    __version__,
    DEFAULT_RENDER_CONFIG,
    InvalidCharacterError,
    draw_code39,
    encode_code39,
    encode_code39_char,
    expand_code39_runs,
    layout_code39,
    normalize_code39,
)
from barcode_layout_1d import Barcode1DError, Barcode1DLayoutConfig, InvalidBarcode1DConfigurationError


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_error_types() -> None:
    assert issubclass(InvalidCharacterError, Barcode1DError)
    assert issubclass(InvalidBarcode1DConfigurationError, Barcode1DError)


def test_normalize_code39() -> None:
    assert normalize_code39("abc-123") == "ABC-123"


def test_encode_char() -> None:
    encoded = encode_code39_char("A")
    assert encoded.pattern == "WNNNNWNNW"


def test_encode_full_sequence() -> None:
    assert [item.char for item in encode_code39("A")] == ["*", "A", "*"]


def test_expand_runs() -> None:
    runs = expand_code39_runs("A")
    assert len(runs) == 29
    assert runs[0].color == "bar"
    assert runs[9].role == "inter-character-gap"
    assert runs[10].modules == 3


def test_paint_scene() -> None:
    scene = draw_code39("A")
    assert scene.metadata["symbology"] == "code39"
    assert scene.width > 0
    assert scene.height == DEFAULT_RENDER_CONFIG.bar_height


def test_invalid_config() -> None:
    try:
        layout_code39("A", Barcode1DLayoutConfig(module_unit=0))
    except InvalidBarcode1DConfigurationError:
        assert True
    else:
        raise AssertionError("expected InvalidBarcode1DConfigurationError")
