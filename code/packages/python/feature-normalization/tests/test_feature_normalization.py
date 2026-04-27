import math

import pytest

from feature_normalization import (
    fit_min_max_scaler,
    fit_standard_scaler,
    transform_min_max,
    transform_standard,
)

ROWS = [
    [1000.0, 3.0, 1.0],
    [1500.0, 4.0, 0.0],
    [2000.0, 5.0, 1.0],
]


def assert_close(expected, actual, tolerance=1e-9):
    assert abs(expected - actual) <= tolerance


def test_standard_scaler_fits_columns():
    scaler = fit_standard_scaler(ROWS)

    assert scaler.means == [1500.0, 4.0, 2.0 / 3.0]
    assert_close(math.sqrt(500000.0 / 3.0), scaler.standard_deviations[0])


def test_standard_transform_centers_and_scales_columns():
    transformed = transform_standard(ROWS, fit_standard_scaler(ROWS))

    assert_close(-1.224744871391589, transformed[0][0])
    assert_close(0.0, transformed[1][0])
    assert_close(1.224744871391589, transformed[2][0])


def test_min_max_transform_maps_columns_to_unit_range():
    transformed = transform_min_max(ROWS, fit_min_max_scaler(ROWS))

    assert transformed == [
        [0.0, 0.0, 1.0],
        [0.5, 0.5, 0.0],
        [1.0, 1.0, 1.0],
    ]


def test_constant_columns_map_to_zero():
    rows = [[1.0, 7.0], [2.0, 7.0]]

    assert transform_standard(rows, fit_standard_scaler(rows)) == [[-1.0, 0.0], [1.0, 0.0]]
    assert transform_min_max(rows, fit_min_max_scaler(rows)) == [[0.0, 0.0], [1.0, 0.0]]


def test_rejects_ragged_matrices():
    with pytest.raises(ValueError):
        fit_standard_scaler([[1.0], [1.0, 2.0]])
