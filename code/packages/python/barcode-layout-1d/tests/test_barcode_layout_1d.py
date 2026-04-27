"""Tests for barcode_layout_1d."""

from barcode_layout_1d import (
    DEFAULT_BARCODE_1D_LAYOUT_CONFIG,
    Barcode1DLayoutConfig,
    InvalidBarcode1DConfigurationError,
    layout_barcode_1d,
    runs_from_binary_pattern,
    runs_from_width_pattern,
)


def test_runs_from_binary_pattern() -> None:
    runs = runs_from_binary_pattern("111001")
    assert [run.color for run in runs] == ["bar", "space", "bar"]
    assert [run.modules for run in runs] == [3, 2, 1]


def test_runs_from_width_pattern() -> None:
    runs = runs_from_width_pattern(
        "WNW",
        ["bar", "space", "bar"],
        source_char="A",
        source_index=0,
    )
    assert [run.modules for run in runs] == [3, 1, 3]


def test_layout_barcode_1d() -> None:
    scene = layout_barcode_1d(
        runs_from_width_pattern(
            "WNW",
            ["bar", "space", "bar"],
            source_char="A",
            source_index=0,
        )
    )
    assert scene.width == (10 + 3 + 1 + 3 + 10) * DEFAULT_BARCODE_1D_LAYOUT_CONFIG.module_unit
    assert scene.height == DEFAULT_BARCODE_1D_LAYOUT_CONFIG.bar_height
    assert len(scene.instructions) == 2


def test_invalid_layout_config() -> None:
    try:
        layout_barcode_1d([], Barcode1DLayoutConfig(module_unit=0))
    except InvalidBarcode1DConfigurationError:
        assert True
    else:
        raise AssertionError("expected InvalidBarcode1DConfigurationError")
