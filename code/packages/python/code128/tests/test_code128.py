from code128 import (
    __version__,
    InvalidCode128InputError,
    compute_code128_checksum,
    draw_code128,
    encode_code128_b,
    expand_code128_runs,
    normalize_code128_b,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_normalize_code128_b_accepts_printable_ascii() -> None:
    assert normalize_code128_b("Code 128") == "Code 128"


def test_normalize_code128_b_rejects_control_characters() -> None:
    try:
        normalize_code128_b("bad\ninput")
    except InvalidCode128InputError:
        assert True
    else:
        raise AssertionError("expected InvalidCode128InputError")


def test_compute_code128_checksum_matches_reference() -> None:
    assert compute_code128_checksum([35, 79, 68, 69, 0, 17, 18, 24]) == 64


def test_encode_code128_b_adds_start_checksum_and_stop() -> None:
    encoded = encode_code128_b("Code 128")
    assert encoded[0].label == "Start B"
    assert encoded[0].role == "start"
    assert encoded[-2].label == "Checksum 64"
    assert encoded[-1].role == "stop"


def test_expand_code128_runs_ends_with_stop_pattern() -> None:
    runs = expand_code128_runs("Hi")
    assert runs[-1].source_char == "Stop"
    assert runs[-1].role == "stop"


def test_draw_code128_returns_barcode_scene() -> None:
    scene = draw_code128("Code 128")
    assert scene.metadata["symbology"] == "code128"
    assert scene.metadata["code_set"] == "B"
    assert scene.metadata["checksum"] == 64
