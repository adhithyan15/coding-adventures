from itf import (
    __version__,
    InvalidItfInputError,
    draw_itf,
    encode_itf,
    expand_itf_runs,
    normalize_itf,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_normalize_itf_accepts_even_length_digit_string() -> None:
    assert normalize_itf("123456") == "123456"


def test_normalize_itf_rejects_odd_length_input() -> None:
    try:
        normalize_itf("12345")
    except InvalidItfInputError:
        assert True
    else:
        raise AssertionError("expected InvalidItfInputError")


def test_encode_itf_encodes_digit_pairs() -> None:
    encoded = encode_itf("123456")
    assert len(encoded) == 3
    assert encoded[0].pair == "12"


def test_expand_itf_runs_include_start_and_stop_patterns() -> None:
    runs = expand_itf_runs("123456")
    assert runs[0].source_char == "start"
    assert runs[0].role == "start"
    assert runs[-1].source_char == "stop"
    assert runs[-1].role == "stop"


def test_draw_itf_returns_barcode_scene() -> None:
    scene = draw_itf("123456")
    assert scene.metadata["symbology"] == "itf"
    assert scene.metadata["pair_count"] == 3
