"""Tests for code39."""

from code39 import (
    __version__,
    DEFAULT_RENDER_CONFIG,
    BarcodeError,
    InvalidCharacterError,
    InvalidConfigurationError,
    draw_code39,
    draw_one_dimensional_barcode,
    encode_code39,
    encode_code39_char,
    expand_code39_runs,
    normalize_code39,
    render_code39,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_error_types() -> None:
    assert issubclass(InvalidCharacterError, BarcodeError)
    assert issubclass(InvalidConfigurationError, BarcodeError)


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
    assert runs[9].is_inter_character_gap is True


def test_draw_scene() -> None:
    scene = draw_code39("A")
    assert scene.metadata["symbology"] == "code39"
    assert scene.width > 0


def test_invalid_config() -> None:
    try:
        draw_one_dimensional_barcode(
            expand_code39_runs("A"),
            "A",
            DEFAULT_RENDER_CONFIG.__class__(wide_unit=4),
        )
    except InvalidConfigurationError:
        assert True
    else:
        raise AssertionError("expected InvalidConfigurationError")


def test_render_with_backend() -> None:
    class DemoRenderer:
        def render(self, scene):  # noqa: ANN001
            return f"{scene.width}:{len(scene.instructions)}"

    assert ":" in render_code39("OK", DemoRenderer())
