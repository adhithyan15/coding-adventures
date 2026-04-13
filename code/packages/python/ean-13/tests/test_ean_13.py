from ean_13 import (
    __version__,
    InvalidEan13CheckDigitError,
    InvalidEan13InputError,
    compute_ean_13_check_digit,
    draw_ean_13,
    encode_ean_13,
    expand_ean_13_runs,
    left_parity_pattern,
    normalize_ean_13,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_compute_ean_13_check_digit_matches_reference() -> None:
    assert compute_ean_13_check_digit("400638133393") == "1"


def test_normalize_ean_13_computes_check_digit() -> None:
    assert normalize_ean_13("400638133393") == "4006381333931"


def test_normalize_ean_13_rejects_non_digit_input() -> None:
    try:
        normalize_ean_13("40063813339A")
    except InvalidEan13InputError:
        assert True
    else:
        raise AssertionError("expected InvalidEan13InputError")


def test_normalize_ean_13_rejects_bad_check_digit() -> None:
    try:
        normalize_ean_13("4006381333932")
    except InvalidEan13CheckDigitError:
        assert True
    else:
        raise AssertionError("expected InvalidEan13CheckDigitError")


def test_left_parity_pattern_matches_reference() -> None:
    assert left_parity_pattern("400638133393") == "LGLLGG"


def test_encode_ean_13_tracks_parity_and_check_digit() -> None:
    encoded = encode_ean_13("400638133393")
    assert encoded[0].digit == "0"
    assert encoded[0].encoding == "L"
    assert encoded[1].encoding == "G"
    assert encoded[-1].role == "check"


def test_expand_ean_13_runs_total_95_modules() -> None:
    runs = expand_ean_13_runs("400638133393")
    assert sum(run.modules for run in runs) == 95


def test_draw_ean_13_returns_scene_metadata() -> None:
    scene = draw_ean_13("400638133393")
    assert scene.metadata["symbology"] == "ean-13"
    assert scene.metadata["left_parity"] == "LGLLGG"
