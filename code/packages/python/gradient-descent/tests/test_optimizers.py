"""Executable examples for the ML02 stochastic-gradient update."""

import pytest

from gradient_descent import sgd


def test_sgd_matches_the_ml02_parity_vector() -> None:
    """Each coordinate takes one independent step opposite its gradient."""
    result = sgd(
        weights=[1.0, -0.5, 2.0],
        gradients=[0.1, -0.2, 0.0],
        learning_rate=0.1,
    )

    assert result == pytest.approx([0.99, -0.48, 2.0])


@pytest.mark.parametrize(
    ("weights", "gradients", "learning_rate", "expected"),
    [
        ([4.0], [2.0], 0.25, [3.5]),
        ([1.0, -2.0], [-3.0, 4.0], 0.5, [2.5, -4.0]),
        ([1.0, -2.0], [-3.0, 4.0], 0.0, [1.0, -2.0]),
        ([1.0, -2.0], [-3.0, 4.0], -0.5, [-0.5, 0.0]),
    ],
)
def test_sgd_applies_the_formula_for_each_coordinate(
    weights: list[float],
    gradients: list[float],
    learning_rate: float,
    expected: list[float],
) -> None:
    assert sgd(weights, gradients, learning_rate) == pytest.approx(expected)


def test_sgd_returns_a_new_list_without_mutating_inputs() -> None:
    weights = [1.0, 2.0]
    gradients = [0.0, 0.0]

    result = sgd(weights, gradients, learning_rate=1.0)

    assert result == weights
    assert result is not weights
    assert weights == [1.0, 2.0]
    assert gradients == [0.0, 0.0]


@pytest.mark.parametrize(
    ("weights", "gradients"),
    [
        ([], []),
        ([1.0], []),
        ([], [1.0]),
        ([1.0], [1.0, 2.0]),
        ([1.0, 2.0], [1.0]),
    ],
)
def test_sgd_rejects_empty_or_differently_sized_vectors(
    weights: list[float], gradients: list[float]
) -> None:
    with pytest.raises(
        ValueError,
        match="Weights and gradients must have the same non-zero length",
    ):
        sgd(weights, gradients, learning_rate=0.1)
