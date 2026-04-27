from codabar import (
    __version__,
    InvalidCodabarInputError,
    draw_codabar,
    encode_codabar,
    expand_codabar_runs,
    normalize_codabar,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_normalize_codabar_defaults_to_a_guards() -> None:
    assert normalize_codabar("40156") == "A40156A"


def test_normalize_codabar_preserves_explicit_guards() -> None:
    assert normalize_codabar("B40156D") == "B40156D"


def test_normalize_codabar_rejects_invalid_body_chars() -> None:
    try:
        normalize_codabar("40*56")
    except InvalidCodabarInputError:
        assert True
    else:
        raise AssertionError("expected InvalidCodabarInputError")


def test_encode_codabar_marks_outer_symbols() -> None:
    encoded = encode_codabar("40156")
    assert encoded[0].char == "A"
    assert encoded[0].role == "start"
    assert encoded[-1].role == "stop"


def test_expand_codabar_runs_adds_inter_character_gaps() -> None:
    runs = expand_codabar_runs("40156")
    assert any(run.role == "inter-character-gap" for run in runs)


def test_draw_codabar_returns_barcode_scene() -> None:
    scene = draw_codabar("40156")
    assert scene.metadata["symbology"] == "codabar"
    assert scene.width > 0
