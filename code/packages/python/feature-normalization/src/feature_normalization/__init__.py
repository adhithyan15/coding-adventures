"""Feature scaling utilities for small machine-learning examples."""

from dataclasses import dataclass
from math import sqrt
from typing import Iterable, List

__version__ = "0.1.0"

Matrix = List[List[float]]


@dataclass(frozen=True)
class StandardScaler:
    means: List[float]
    standard_deviations: List[float]


@dataclass(frozen=True)
class MinMaxScaler:
    minimums: List[float]
    maximums: List[float]


def _validate_matrix(rows: Iterable[Iterable[float]]) -> Matrix:
    matrix = [[float(value) for value in row] for row in rows]
    if not matrix or not matrix[0]:
        raise ValueError("matrix must have at least one row and one column")

    width = len(matrix[0])
    if any(len(row) != width for row in matrix):
        raise ValueError("all rows must have the same number of columns")
    return matrix


def fit_standard_scaler(rows: Iterable[Iterable[float]]) -> StandardScaler:
    matrix = _validate_matrix(rows)
    count = len(matrix)
    width = len(matrix[0])
    means = [sum(row[col] for row in matrix) / count for col in range(width)]
    variances = [
        sum((row[col] - means[col]) ** 2 for row in matrix) / count
        for col in range(width)
    ]
    return StandardScaler(means, [sqrt(value) for value in variances])


def transform_standard(rows: Iterable[Iterable[float]], scaler: StandardScaler) -> Matrix:
    matrix = _validate_matrix(rows)
    if len(matrix[0]) != len(scaler.means):
        raise ValueError("matrix width must match scaler width")

    transformed: Matrix = []
    for row in matrix:
        transformed.append([
            0.0 if scaler.standard_deviations[col] == 0.0
            else (row[col] - scaler.means[col]) / scaler.standard_deviations[col]
            for col in range(len(row))
        ])
    return transformed


def fit_transform_standard(rows: Iterable[Iterable[float]]) -> tuple[Matrix, StandardScaler]:
    scaler = fit_standard_scaler(rows)
    return transform_standard(rows, scaler), scaler


def fit_min_max_scaler(rows: Iterable[Iterable[float]]) -> MinMaxScaler:
    matrix = _validate_matrix(rows)
    width = len(matrix[0])
    return MinMaxScaler(
        [min(row[col] for row in matrix) for col in range(width)],
        [max(row[col] for row in matrix) for col in range(width)],
    )


def transform_min_max(rows: Iterable[Iterable[float]], scaler: MinMaxScaler) -> Matrix:
    matrix = _validate_matrix(rows)
    if len(matrix[0]) != len(scaler.minimums):
        raise ValueError("matrix width must match scaler width")

    transformed: Matrix = []
    for row in matrix:
        transformed.append([
            0.0 if scaler.maximums[col] == scaler.minimums[col]
            else (row[col] - scaler.minimums[col]) / (scaler.maximums[col] - scaler.minimums[col])
            for col in range(len(row))
        ])
    return transformed


def fit_transform_min_max(rows: Iterable[Iterable[float]]) -> tuple[Matrix, MinMaxScaler]:
    scaler = fit_min_max_scaler(rows)
    return transform_min_max(rows, scaler), scaler
