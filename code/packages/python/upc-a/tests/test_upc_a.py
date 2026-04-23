from upc_a import (
    __version__,
    InvalidUpcACheckDigitError,
    InvalidUpcAInputError,
    compute_upc_a_check_digit,
    draw_upc_a,
    encode_upc_a,
    expand_upc_a_runs,
    normalize_upc_a,
)


def test_version_exists() -> None:
    assert __version__ == "0.1.0"


def test_compute_upc_a_check_digit_matches_reference() -> None:
    assert compute_upc_a_check_digit("03600029145") == "2"


def test_normalize_upc_a_computes_check_digit() -> None:
    assert normalize_upc_a("03600029145") == "036000291452"


def test_normalize_upc_a_rejects_non_digit_input() -> None:
    try:
        normalize_upc_a("03600A29145")
    except InvalidUpcAInputError:
        assert True
    else:
        raise AssertionError("expected InvalidUpcAInputError")


def test_normalize_upc_a_rejects_bad_check_digit() -> None:
    try:
        normalize_upc_a("036000291453")
    except InvalidUpcACheckDigitError:
        assert True
    else:
        raise AssertionError("expected InvalidUpcACheckDigitError")


def test_encode_upc_a_marks_final_digit_as_check() -> None:
    encoded = encode_upc_a("03600029145")
    assert len(encoded) == 12
    assert encoded[0].encoding == "L"
    assert encoded[-1].digit == "2"
    assert encoded[-1].role == "check"


def test_expand_upc_a_runs_total_95_modules() -> None:
    runs = expand_upc_a_runs("03600029145")
    assert sum(run.modules for run in runs) == 95
    assert runs[0].role == "guard"


def test_draw_upc_a_returns_barcode_scene() -> None:
    scene = draw_upc_a("03600029145")
    assert scene.metadata["symbology"] == "upc-a"
    assert scene.metadata["content_modules"] == 95
